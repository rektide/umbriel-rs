use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    sync::mpsc,
};
use umbriel_client::{Endpoint, Observation, SubscriptionOptions};
use umbriel_protocol::{envelope::Family, model::Windows};
use umbriel_state::{active::StableActiveReducer, inventory::Health};
use umbriel_tracking::{
    Tracker, Update,
    activity::{Activity, Event},
};

fn frame(id: &str, title: &str) -> String {
    format!(
        r#"{{"event":"windows","data":[{{"id":"{id}","workspace":"ws","scratchpad":"","active":true,"app_id":"app","title":"{title}","xdg_tag":"","content_type":"none","floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}]}}
"#
    )
}
fn item(epoch: u64, position: u64, text: &str) -> Observation {
    Observation::Frame {
        epoch,
        position,
        bytes: Arc::from(text.trim_end().as_bytes()),
    }
}
fn rows(id: &str, title: &str) -> Windows {
    let text = frame(id, title);
    let umbriel_protocol::envelope::Message::Event { data, .. } = umbriel_protocol::envelope::Message::decode(text.trim().as_bytes()).unwrap() else {
        panic!()
    };
    umbriel_protocol::codec::decode(data.as_raw_str().as_bytes()).unwrap()
}

#[test]
fn metadata_is_owned_by_accepted_candidate_not_last_reported_identity() {
    let mut reducer = StableActiveReducer::new(Duration::from_secs(5));
    let now = Duration::from_secs;
    assert_eq!(reducer.observe_changes(1, now(0), &rows("A", "first")).unwrap(), None);
    assert_eq!(reducer.observe_changes(1, now(1), &rows("A", "before")).unwrap(), None);
    assert_eq!(reducer.poll(now(5)).unwrap().unwrap().window.unwrap().title, "before");
    let metadata = reducer.observe_changes(1, now(6), &rows("A", "after")).unwrap().unwrap();
    assert_eq!(metadata.since, now(0));
    assert_eq!(metadata.window.unwrap().title, "after");
    let mut geometry = rows("A", "after");
    geometry[0].x = 99;
    assert_eq!(reducer.observe_changes(1, now(7), &geometry).unwrap(), None);
    reducer.observe_changes(1, now(8), &rows("B", "other")).unwrap();
    assert_eq!(reducer.observe_changes(1, now(9), &rows("A", "returned")).unwrap(), None);
    assert_eq!(reducer.poll(now(13)).unwrap(), None);
    assert!(reducer.poll(now(14)).unwrap().is_some());
    reducer.observe_changes(1, now(15), &[]).unwrap();
    assert_eq!(reducer.observe_changes(1, now(16), &rows("A", "again")).unwrap(), None);
    reducer.invalidate();
    assert_eq!(reducer.observe_changes(1, now(22), &rows("A", "after gap")).unwrap(), None);
    assert_eq!(reducer.poll(now(22)).unwrap(), None);
}

#[test]
fn family_failure_keeps_other_family_current_and_retains_stale_rows() {
    let mut tracker = Tracker::new(&[Family::Windows, Family::Submap], true);
    tracker.observe(Duration::ZERO, Observation::Epoch { epoch: 1 });
    tracker.observe(Duration::ZERO, item(1, 1, &frame("A", "old")));
    tracker.observe(Duration::ZERO, item(1, 2, r#"{"event":"submap","data":"default"}"#));
    assert_eq!(tracker.inventory().status(Family::Submap).health, Health::Current);
    assert!(matches!(
        tracker.observe(Duration::ZERO, item(1, 3, r#"{"event":"windows","data":null}"#)),
        Update::Rejected {
            family: Some(Family::Windows),
            ..
        }
    ));
    assert_eq!(tracker.inventory().status(Family::Submap).health, Health::Current);
    assert!(!tracker.inventory().status(Family::Windows).is_ready());
    assert_eq!(tracker.inventory().snapshot().windows.entries().len(), 1);
    let Update::Installed { windows: Some(changes), .. } = tracker.observe(Duration::ZERO, item(1, 4, &frame("B", "new"))) else {
        panic!()
    };
    assert!(!changes.is_continuous());
    tracker.lost();
    assert!(matches!(
        tracker.observe(Duration::ZERO, item(1, 5, &frame("C", "lost"))),
        Update::Rejected { .. }
    ));
    tracker.observe(Duration::ZERO, Observation::Epoch { epoch: 2 });
    assert!(matches!(
        tracker.observe(Duration::ZERO, item(2, 1, &frame("C", "fresh"))),
        Update::Installed { .. }
    ));
}

#[tokio::test(start_paused = true)]
async fn queued_transition_beats_due_timer_and_timer_needs_no_new_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("peer.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (send, mut receive) = mpsc::channel::<String>(4);
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (read, mut write) = socket.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        write.write_all(frame("A", "first").as_bytes()).await.unwrap();
        while let Some(frame) = receive.recv().await {
            write.write_all(frame.as_bytes()).await.unwrap();
        }
    });
    let (source, mut observer) = Endpoint::new(path).subscribe(&[Family::Windows], SubscriptionOptions::default()).await.unwrap();
    let mut activity = Activity::new(Duration::from_millis(750));
    while !matches!(activity.next(&mut observer).await.unwrap(), Event::Ready { .. }) {}
    let mut tap = source.observe().await.unwrap();
    tokio::time::advance(Duration::from_millis(750)).await;
    send.send(frame("B", "second")).await.unwrap();
    loop {
        if matches!(tap.recv().await.unwrap(), Observation::Frame { .. }) {
            break;
        }
    }
    assert!(tokio::time::timeout(Duration::from_millis(749), activity.next(&mut observer)).await.is_err());
    tokio::time::advance(Duration::from_millis(1)).await;
    let Event::Accepted(stable) = activity.next(&mut observer).await.unwrap() else {
        panic!()
    };
    assert_eq!(stable.window.unwrap().id.as_str(), "B");
    send.send(frame("B", "updated")).await.unwrap();
    let Event::Metadata(update) = activity.next(&mut observer).await.unwrap() else {
        panic!()
    };
    assert_eq!(update.since, stable.since);
    assert_eq!(update.window.unwrap().title, "updated");
    source.shutdown().await;
    assert!(matches!(activity.next(&mut observer).await.unwrap(), Event::Gap { .. }));
    assert!(activity.next(&mut observer).await.is_err());
    assert!(!activity.tracker().inventory().status(Family::Windows).is_ready());
    drop(send);
    peer.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn terminal_observer_lag_beats_due_acceptance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lag.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (send, mut receive) = mpsc::channel::<String>(4);
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (read, mut write) = socket.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        write.write_all(frame("A", "first").as_bytes()).await.unwrap();
        while let Some(frame) = receive.recv().await {
            write.write_all(frame.as_bytes()).await.unwrap();
        }
    });
    let options = SubscriptionOptions {
        observer_capacity: 2,
        ..Default::default()
    };
    let (source, mut observer) = Endpoint::new(path).subscribe(&[Family::Windows], options).await.unwrap();
    let mut activity = Activity::new(Duration::from_secs(5));
    while !matches!(activity.next(&mut observer).await.unwrap(), Event::Ready { .. }) {}
    let mut tap = source.observe().await.unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    for id in ["B", "A", "C"] {
        send.send(frame(id, "queued")).await.unwrap();
        loop {
            if matches!(tap.recv().await.unwrap(), Observation::Frame { .. }) {
                break;
            }
        }
    }
    assert!(matches!(
        activity.next(&mut observer).await,
        Err(umbriel_tracking::activity::Error::Delivery(umbriel_client::DeliveryEnd::Lagged { .. }))
    ));
    assert!(!activity.tracker().inventory().status(Family::Windows).is_ready());
    source.shutdown().await;
    drop(send);
    peer.await.unwrap();
}
