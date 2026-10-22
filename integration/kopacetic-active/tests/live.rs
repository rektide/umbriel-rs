use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    sync::mpsc,
};
use umbriel_client::{DeliveryEnd, Endpoint, Observation, Reconnect, SubscriptionOptions};
use umbriel_kopacetic_active::{ActiveCursor, CursorFeed, CursorReader, CursorState, Entity};
use umbriel_protocol::envelope::Family;
use umbriel_session::{EndReason, Terminal};

fn window(id: &str, title: &str, active: bool) -> String {
    format!(
        r#"{{"id":"{id}","workspace":"ws","scratchpad":"","active":{active},"app_id":"app","title":"{title}","xdg_tag":"","content_type":"none","floating":false,"focused":true,"urgent":false,"xwayland":false,"pid":-1,"x":-20,"y":0,"w":100,"h":200}}"#
    )
}

fn frame(windows: &str) -> String {
    format!("{{\"event\":\"windows\",\"data\":[{windows}]}}\n")
}

fn observation(epoch: u64, position: u64, bytes: &str) -> Observation {
    Observation::Frame {
        epoch,
        position,
        bytes: Arc::from(bytes.trim_end().as_bytes()),
    }
}

fn selected(reader: &mut CursorReader) -> String {
    match reader.snapshot().payload.state {
        CursorState::Active(window) => window.id,
        state => panic!("expected active, got {state:?}"),
    }
}

#[test]
fn entity_identity_stays_stable_while_active_and_metadata_change() {
    fn reflected<T: facet::Facet<'static>>() {}
    reflected::<Entity<ActiveCursor>>();
    assert!(CursorFeed::channel(" ").is_err());
    let (mut feed, mut reader) = CursorFeed::channel("seat@socket").unwrap();
    let original = reader.snapshot();
    assert_eq!(original.identity.name, "seat@socket");
    assert_eq!(original.identity.kind, "UmbrielActiveCursor.v1");
    assert_eq!(original.payload.state, CursorState::AwaitingCensus);
    feed.observe(&observation(1, 1, &frame(&window("A", "old", true))));
    let historical = reader.snapshot();
    feed.observe(&observation(1, 2, &frame(&window("A", "new", true))));
    match reader.snapshot().payload.state {
        CursorState::Active(window) => {
            assert_eq!(window.title, "new");
            assert_eq!((window.pid, window.x, window.h), (-1, -20, 200));
        }
        state => panic!("{state:?}"),
    }
    feed.observe(&observation(1, 3, &frame(&window("B", "next", true))));
    assert_eq!(selected(&mut reader), "B");
    assert_eq!(reader.snapshot().identity, original.identity);
    assert_eq!(historical.payload.position, Some(1));
    feed.observe(&observation(1, 4, &frame(&window("B", "remembered focus", false))));
    assert_eq!(reader.snapshot().payload.state, CursorState::NoActive);
    feed.observe(&observation(1, 5, &frame("")));
    assert_eq!(reader.snapshot().payload.state, CursorState::NoActive);
}

#[test]
fn invalid_loss_and_reconnect_clear_active_instead_of_fabricating_no_active() {
    let (mut feed, mut reader) = CursorFeed::channel("seat").unwrap();
    let active = frame(&window("A", "title", true));
    for invalid in [
        "not json".to_owned(),
        r#"{"event":"windows","data":null}"#.into(),
        frame(&window("", "empty ID", true)),
        frame(&format!("{},{}", window("A", "one", true), window("B", "two", true))),
    ] {
        feed.observe(&observation(1, 1, &active));
        feed.observe(&observation(1, 2, &invalid));
        assert!(matches!(reader.snapshot().payload.state, CursorState::Invalid(_)));
    }
    feed.observe(&observation(1, 3, &active));
    assert_eq!(selected(&mut reader), "A");
    feed.observe(&Observation::Gap {
        epoch: 1,
        terminal: Terminal {
            reason: EndReason::TransportLost("test".into()),
            unresolved: None,
        },
    });
    assert!(matches!(reader.snapshot().payload.state, CursorState::Gap(_)));
    feed.observe(&Observation::Epoch { epoch: 2 });
    assert_eq!(reader.snapshot().payload.state, CursorState::AwaitingCensus);
    assert_eq!(reader.snapshot().payload.position, None);
    feed.observe(&observation(2, 1, &active));
    feed.observe(&observation(2, 2, r#"{"event":"future","data":{}}"#));
    assert_eq!(selected(&mut reader), "A");
    assert_eq!(reader.snapshot().payload.position, Some(1));
    feed.observe(&observation(3, 1, r#"{"event":"future","data":{}}"#));
    assert_eq!(reader.snapshot().payload.state, CursorState::AwaitingCensus);
    for error in [DeliveryEnd::DriverLost, DeliveryEnd::Lagged { epoch: 3, position: 9 }] {
        let (mut feed, mut reader) = CursorFeed::channel("seat").unwrap();
        feed.observe(&observation(3, 8, &active));
        feed.end(&error);
        assert!(!feed.observe(&observation(4, 1, &active)));
        drop(feed);
        assert!(matches!(reader.snapshot().payload.state, CursorState::Ended(_)));
        assert_eq!(reader.snapshot().payload.position, None);
    }
}

#[tokio::test]
async fn slow_readers_coalesce_and_drop_publishes_stopped_before_close() {
    let (mut feed, mut reader) = CursorFeed::channel("seat").unwrap();
    for position in 1..=100 {
        feed.observe(&observation(1, position, &frame(&window("A", "title", true))));
    }
    reader.changed().await.unwrap();
    assert_eq!(reader.snapshot().payload.position, Some(100));
    drop(feed);
    reader.changed().await.unwrap();
    assert_eq!(reader.snapshot().payload.state, CursorState::Stopped);
    assert!(reader.changed().await.is_err());
}

async fn wait_for(reader: &mut CursorReader, predicate: impl Fn(&CursorState) -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if predicate(&reader.snapshot().payload.state) {
                return;
            }
            reader.changed().await.unwrap();
        }
    })
    .await
    .expect("cursor transition");
}

#[tokio::test]
async fn native_subscription_reconnect_cancellation_and_source_shutdown() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (commands, mut receive) = mpsc::channel::<String>(4);
    let (connections, mut connected) = mpsc::channel(2);
    let peer = tokio::spawn(async move {
        for _ in 0..2 {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let mut line = String::new();
            BufReader::new(read).read_line(&mut line).await.unwrap();
            assert!(line.contains("subscribe"));
            connections.send(()).await.unwrap();
            while let Some(message) = receive.recv().await {
                if message == "disconnect" {
                    break;
                }
                write.write_all(message.as_bytes()).await.unwrap();
            }
        }
    });
    let (source, observer) = Endpoint::new(&path)
        .subscribe(
            &[Family::Windows],
            SubscriptionOptions {
                reconnect: Reconnect::Retry {
                    delay: Duration::from_millis(10),
                    max_attempts: 2,
                },
                ..SubscriptionOptions::default()
            },
        )
        .await
        .unwrap();
    let (feed, mut reader) = CursorFeed::channel("scripted").unwrap();
    let task = tokio::spawn(feed.follow(observer));
    connected.recv().await.unwrap();
    commands.send(frame(&window("A", "first", true))).await.unwrap();
    wait_for(&mut reader, |state| matches!(state, CursorState::Active(w) if w.id == "A")).await;
    let first_epoch = reader.snapshot().payload.epoch;
    commands.send("disconnect".into()).await.unwrap();
    connected.recv().await.unwrap();
    wait_for(&mut reader, |state| matches!(state, CursorState::AwaitingCensus)).await;
    assert_ne!(reader.snapshot().payload.epoch, first_epoch);
    commands.send(frame(&window("B", "second", true))).await.unwrap();
    wait_for(&mut reader, |state| matches!(state, CursorState::Active(w) if w.id == "B")).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(reader.snapshot().payload.state, CursorState::Stopped);

    // A separate observer's terminal source failure is also published, even
    // though the original reader was cancelled independently of the source.
    let observer = source.observe().await.unwrap();
    let (feed, mut terminal_reader) = CursorFeed::channel("scripted-terminal").unwrap();
    let task = tokio::spawn(feed.follow(observer));
    source.shutdown().await;
    let _end = task.await.unwrap();
    assert!(matches!(terminal_reader.snapshot().payload.state, CursorState::Ended(_)));
    drop(commands);
    peer.await.unwrap();
}
