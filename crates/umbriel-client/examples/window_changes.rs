//! Read-only native window census differences and related workspace impact.
use kubeless_view_impact::{affected_roots, visit_relations};
use std::{error::Error, fmt::Write, time::Duration};
use tokio::time::Instant;
use umbriel_client::{Discovery, Observation, SubscriptionOptions};
use umbriel_protocol::{envelope::Family, model::Window};
use umbriel_state::inventory::WindowChanges;
use umbriel_tracking::{Tracker, Update};

/// Empty references are not workspace identities. Nonempty references need not
/// match a separately observed workspace census, so no existence is inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Relation<'a> {
    Workspace(&'a str),
    Scratchpad(&'a str),
    NoWorkspace,
}

fn relation(window: &Window) -> Relation<'_> {
    if !window.workspace.as_str().is_empty() {
        Relation::Workspace(window.workspace.as_str())
    } else if !window.scratchpad.is_empty() {
        Relation::Scratchpad(&window.scratchpad)
    } else {
        Relation::NoWorkspace
    }
}

fn report(mut installed: WindowChanges) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "window census: before={:?} after={:?} local_generation={} comparison={}",
        installed.before,
        installed.after,
        installed.commit.generation.get(),
        if installed.is_continuous() {
            "uninterrupted observations"
        } else {
            "baseline/reacquired retained snapshots; not continuous history"
        }
    )
    .unwrap();
    // Equality is native full typed Window equality, not Arc identity or only
    // workspace membership. Keep the actual receipt's generation when filtering.
    installed.commit.evidence.retain(|row| row.before != row.after);
    let changes = &installed.commit.evidence;
    let mut rows = changes.iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| a.key.cmp(&b.key)); // display order, not compositor order
    for row in rows {
        let kind = match (&row.before, &row.after) {
            (None, Some(_)) => "added",
            (Some(_), None) => "removed",
            _ => "changed",
        };
        writeln!(
            out,
            "{kind} {}: {:?} -> {:?}",
            row.key,
            row.before.as_ref().map(|w| (&w.title, relation(w))),
            row.after.as_ref().map(|w| (&w.title, relation(w)))
        )
        .unwrap();
    }
    // Both sides matter: a move A -> B invalidates A as well as B. Traversal
    // also exposes the window/side provenance that deduplicated roots drop.
    visit_relations(
        changes,
        |_, w| [relation(w)],
        |id, side, root| {
            writeln!(out, "relation {id} {side:?}: {root:?}").unwrap();
        },
    );
    let roots = affected_roots(changes, |_, w| [relation(w)]);
    writeln!(out, "affected relations (unordered): {:?}; changed windows={}", roots.keys(), changes.len()).unwrap();
    out
}

fn observe(tracker: &mut Tracker, now: Duration, observation: Observation) -> Result<Option<String>, Box<dyn Error>> {
    match tracker.observe(now, observation) {
        Update::Epoch { epoch } => Ok(Some(format!("epoch {epoch}: retained rows are stale; reacquiring"))),
        Update::Gap { epoch, reason } => Ok(Some(format!("epoch {epoch}: source gap {reason}; no continuous history"))),
        Update::Installed { windows: Some(receipt), .. } => Ok(Some(report(receipt))),
        Update::Rejected { reason, .. } => Ok(Some(format!(
            "decode gap/rejected census: {reason}; {:?}",
            tracker.inventory().status(Family::Windows)
        ))),
        _ => Ok(None),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let endpoint = Discovery::from_env().resolve()?;
    let (source, mut observer) = endpoint.subscribe(&[Family::Windows], SubscriptionOptions::default()).await?;
    let origin = Instant::now();
    let mut inventory = Tracker::new(&[Family::Windows], true);
    let result = async {
        loop {
            let item = match observer.recv().await {
                Ok(item) => item,
                Err(error) => {
                    inventory.lost();
                    eprintln!("observer ended/lost delivery: {error}; {:?}", inventory.inventory().status(Family::Windows));
                    return Err(Box::<dyn Error>::from(error));
                }
            };
            if let Some(report) = observe(&mut inventory, origin.elapsed(), item)? {
                println!("{report}");
            }
        }
    }
    .await;
    // Keep the native owner alive; raw delivery retains its byte/loss contracts.
    source.shutdown().await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };
    use umbriel_client::Endpoint;

    fn frame(workspace: &str, scratchpad: &str) -> String {
        format!(
            r#"{{"event":"windows","data":[{{"id":"A","workspace":"{workspace}","scratchpad":"{scratchpad}","active":true,"app_id":"app","title":"title","xdg_tag":"","content_type":"none","floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}]}}
"#
        )
    }

    #[tokio::test]
    async fn scripted_native_peer_reports_move_both_sides_and_equal_census() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("changes.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (done, wait) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let mut line = String::new();
            BufReader::new(read).read_line(&mut line).await.unwrap();
            assert!(line.contains("subscribe"));
            let frames = [
                frame("old", ""),
                frame("new", ""),
                frame("new", ""),
                frame("", "pad"),
                frame("", ""),
                "{\"event\":\"windows\",\"data\":[]}\n".into(),
            ];
            for frame in frames {
                write.write_all(frame.as_bytes()).await.unwrap();
            }
            let _ = wait.await; // do not turn pending frames into a transport loss
        });
        let (source, mut observer) = Endpoint::new(path).subscribe(&[Family::Windows], SubscriptionOptions::default()).await.unwrap();
        let mut inventory = Tracker::new(&[Family::Windows], true);
        let mut reports = Vec::new();
        while reports.len() < 6 {
            let item = tokio::time::timeout(Duration::from_secs(5), observer.recv()).await.unwrap().unwrap();
            let is_frame = matches!(item, Observation::Frame { .. });
            let report = observe(&mut inventory, Duration::ZERO, item).unwrap();
            if is_frame {
                reports.push(report.unwrap());
            }
        }
        assert!(reports[0].contains("baseline/reacquired"));
        assert!(reports[0].contains("added A"));
        assert!(reports[1].contains("uninterrupted observations"));
        assert!(reports[1].contains("relation A Before: Workspace(\"old\")"));
        assert!(reports[1].contains("relation A After: Workspace(\"new\")"));
        assert!(reports[1].contains("[Workspace(\"old\"), Workspace(\"new\")]"));
        assert!(reports[2].contains("changed windows=0"));
        assert!(reports[2].contains("affected relations (unordered): []"));
        assert!(reports[3].contains("Scratchpad(\"pad\")"));
        assert!(!reports[3].contains("Workspace(\"\")"));
        assert!(reports[4].contains("NoWorkspace"));
        assert!(reports[5].contains("removed A"));
        source.shutdown().await;
        let _ = done.send(());
        peer.await.unwrap();
    }

    #[test]
    fn adapter_decode_failure_and_epoch_do_not_hide_retained_evidence() {
        use std::sync::Arc;
        let mut inventory = Tracker::new(&[Family::Windows], true);
        observe(&mut inventory, Duration::ZERO, Observation::Epoch { epoch: 1 }).unwrap();
        let item = |epoch, position, text: String| Observation::Frame {
            epoch,
            position,
            bytes: Arc::from(text.trim_end().as_bytes()),
        };
        observe(&mut inventory, Duration::ZERO, item(1, 1, frame("old", ""))).unwrap();
        let failed = observe(&mut inventory, Duration::ZERO, item(1, 2, "{\"event\":\"windows\",\"data\":null}".into()))
            .unwrap()
            .unwrap();
        assert!(failed.contains("decode gap"));
        assert!(failed.contains("InvalidSnapshot"));
        let resumed = observe(&mut inventory, Duration::ZERO, item(1, 3, frame("new", ""))).unwrap().unwrap();
        assert!(resumed.contains("baseline/reacquired"));
        assert!(resumed.contains("Workspace(\"old\")"));
        observe(&mut inventory, Duration::ZERO, Observation::Epoch { epoch: 2 }).unwrap();
        let reacquired = observe(&mut inventory, Duration::ZERO, item(2, 1, frame("next", ""))).unwrap().unwrap();
        assert!(reacquired.contains("EpochChanged"));
        assert!(reacquired.contains("baseline/reacquired"));
    }
}
