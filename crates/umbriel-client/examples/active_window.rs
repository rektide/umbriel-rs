//! Native read-only observation: never sleeps in the socket receive path.
use std::error::Error;
use tokio::time::{Instant, sleep_until};
use umbriel_client::{Discovery, Observation, Observer, SubscriptionOptions};
use umbriel_protocol::{
    envelope::{EventName, Family, Message},
    model::Windows,
};
use umbriel_state::active::{StableActive, StableActiveReducer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let endpoint = Discovery::from_env().resolve()?;
    let (source, mut observer) = endpoint
        .subscribe(&[Family::Windows], SubscriptionOptions::default())
        .await?;
    let origin = Instant::now();
    let mut reducer = StableActiveReducer::default();
    let result = async {
        loop {
            let stable = next_stable(&mut observer, &mut reducer, origin).await?;
            match stable.window {
                Some(window) => println!("{}\t{}\t{}", window.id, window.app_id, window.title),
                None => println!("(no active window)"),
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn Error>>(())
    }
    .await;
    // Observers alone do not keep the source alive. Retain its owner throughout.
    source.shutdown().await;
    result
}

fn observe(reducer: &mut StableActiveReducer, origin: Instant, observation: Observation) {
    match observation {
        Observation::Epoch { epoch } => {
            reducer.invalidate();
            eprintln!("epoch {epoch}: starting fresh observation continuity");
        }
        Observation::Gap { epoch, terminal } => {
            reducer.invalidate();
            eprintln!("epoch {epoch}: source gap: {terminal:?}");
        }
        Observation::Frame { epoch, bytes, .. } => {
            // Borrow the raw frame; retain only selected candidate metadata.
            let decoded = Message::decode(&bytes).and_then(|message| {
                if let Message::Event {
                    name: EventName::Known(Family::Windows),
                    data,
                } = message
                {
                    umbriel_protocol::codec::decode::<Windows>(data.as_raw_str().as_bytes())
                        .map(Some)
                } else {
                    Ok(None)
                }
            });
            match decoded {
                Ok(Some(windows)) => {
                    if let Err(error) = reducer.observe(epoch, origin.elapsed(), &windows) {
                        eprintln!("invalid census: {error}");
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    reducer.invalidate();
                    eprintln!("decode gap: {error}");
                }
            }
        }
    }
}

async fn next_stable(
    observer: &mut Observer,
    reducer: &mut StableActiveReducer,
    origin: Instant,
) -> Result<StableActive, Box<dyn Error>> {
    loop {
        // A due timer may not bypass a queued B (or A-B-A), gap, or terminal lag.
        // Bound work turns, but postpone publication whenever that budget is used.
        let mut caught_up = false;
        for _ in 0..64 {
            match observer.try_recv() {
                Ok(Some(item)) => observe(reducer, origin, item),
                Ok(None) => {
                    caught_up = true;
                    break;
                }
                Err(error) => {
                    reducer.invalidate();
                    return Err(error.into());
                }
            }
        }
        if !caught_up {
            tokio::task::yield_now().await;
            continue;
        }
        if let Some(stable) = reducer.poll(origin.elapsed())? {
            return Ok(stable);
        }
        if let Some(deadline) = reducer.deadline() {
            let deadline = origin
                .checked_add(deadline)
                .ok_or("runtime deadline overflow")?;
            tokio::select! {
                biased;
                item = observer.recv() => {
                    match item {
                        Ok(item) => observe(reducer, origin, item),
                        Err(error) => { reducer.invalidate(); return Err(error.into()); }
                    }
                }
                _ = sleep_until(deadline) => {}
            }
        } else {
            match observer.recv().await {
                Ok(item) => observe(reducer, origin, item),
                Err(error) => {
                    reducer.invalidate();
                    return Err(error.into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
        sync::mpsc,
    };
    use umbriel_client::Endpoint;

    fn frame(id: &str) -> String {
        format!(
            r#"{{"event":"windows","data":[{{"id":"{id}","workspace":"ws","scratchpad":"","active":true,"app_id":"app","title":"title","xdg_tag":"","content_type":"none","floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}]}}
"#
        )
    }

    #[tokio::test(start_paused = true)]
    async fn native_timer_emits_without_more_input_but_queued_transition_wins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scripted.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (send, mut receive) = mpsc::channel::<String>(4);
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let mut line = String::new();
            BufReader::new(read).read_line(&mut line).await.unwrap();
            assert!(line.contains("subscribe"));
            write.write_all(frame("A").as_bytes()).await.unwrap();
            while let Some(frame) = receive.recv().await {
                write.write_all(frame.as_bytes()).await.unwrap();
            }
        });
        let (source, mut observer) = Endpoint::new(path)
            .subscribe(&[Family::Windows], SubscriptionOptions::default())
            .await
            .unwrap();
        let origin = Instant::now();
        let mut reducer = StableActiveReducer::default();
        // Establish initial A at the consumer's observation time, not a prior duration.
        loop {
            let item = observer.recv().await.unwrap();
            let is_frame = matches!(item, Observation::Frame { .. });
            observe(&mut reducer, origin, item);
            if is_frame {
                break;
            }
        }
        let tap = source.observe().await.unwrap();
        let mut tap = tap;
        tokio::time::advance(Duration::from_millis(750)).await;
        send.send(frame("B")).await.unwrap();
        // This independently delivered frame proves B is already queued before
        // the primary consumer considers its due A timer.
        loop {
            if matches!(tap.recv().await.unwrap(), Observation::Frame { .. }) {
                break;
            }
        }
        let result = tokio::time::timeout(
            Duration::from_millis(749),
            next_stable(&mut observer, &mut reducer, origin),
        )
        .await;
        assert!(result.is_err(), "queued B must cancel due A, not report it");
        tokio::time::advance(Duration::from_millis(1)).await;
        let stable = next_stable(&mut observer, &mut reducer, origin)
            .await
            .unwrap();
        assert_eq!(stable.window.unwrap().id.as_str(), "B");
        assert_eq!(stable.since, Duration::from_millis(750));
        assert_eq!(stable.emitted_at, Duration::from_millis(1500));
        source.shutdown().await;
        drop(send);
        peer.await.unwrap();
    }

    #[test]
    fn epoch_gap_and_decode_failure_never_become_none() {
        use std::sync::Arc;
        let origin = Instant::now();
        let mut reducer = StableActiveReducer::default();
        observe(
            &mut reducer,
            origin,
            Observation::Frame {
                epoch: 1,
                position: 1,
                bytes: Arc::from(frame("A").trim_end().as_bytes()),
            },
        );
        assert!(reducer.deadline().is_some());
        observe(&mut reducer, origin, Observation::Epoch { epoch: 2 });
        assert_eq!(reducer.deadline(), None);
        observe(
            &mut reducer,
            origin,
            Observation::Frame {
                epoch: 2,
                position: 2,
                bytes: Arc::from(frame("A").trim_end().as_bytes()),
            },
        );
        assert!(reducer.deadline().is_some());
        observe(
            &mut reducer,
            origin,
            Observation::Gap {
                epoch: 2,
                terminal: umbriel_session::Terminal {
                    reason: umbriel_session::EndReason::TransportLost("scripted".into()),
                    unresolved: None,
                },
            },
        );
        assert_eq!(reducer.deadline(), None);
        observe(
            &mut reducer,
            origin,
            Observation::Frame {
                epoch: 2,
                position: 1,
                bytes: Arc::from(&b"{\"event\":\"windows\",\"data\":null}"[..]),
            },
        );
        assert_eq!(reducer.deadline(), None);
    }
}
