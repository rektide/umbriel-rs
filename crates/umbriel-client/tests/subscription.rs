use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{mpsc, oneshot},
};
use umbriel_client::{
    DeliveryEnd, Endpoint, Error, Health, Observation, Reconnect, Subscription, SubscriptionOptions,
};
use umbriel_protocol::{
    envelope::{Family, Message},
    request::{Query, Request},
};
use umbriel_session::{EndReason, FamilyStatus};

enum Script {
    Write(Vec<u8>, oneshot::Sender<()>),
    Read(oneshot::Sender<String>),
    Eof(oneshot::Sender<()>),
}
struct Peer(mpsc::Sender<Script>);
impl Peer {
    async fn write(&self, bytes: &[u8]) {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(Script::Write(bytes.to_vec(), tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }
    async fn read(&self) -> String {
        let (tx, rx) = oneshot::channel();
        self.0.send(Script::Read(tx)).await.unwrap();
        rx.await.unwrap()
    }
    async fn eof(&self) {
        let (tx, rx) = oneshot::channel();
        self.0.send(Script::Eof(tx)).await.unwrap();
        rx.await.unwrap();
    }
}
async fn accept(listener: &UnixListener) -> Peer {
    let (socket, _) = listener.accept().await.unwrap();
    peer(socket)
}
fn peer(socket: UnixStream) -> Peer {
    let (tx, mut rx) = mpsc::channel(8);
    tokio::spawn(async move {
        let mut socket = BufReader::new(socket);
        while let Some(script) = rx.recv().await {
            match script {
                Script::Write(bytes, tx) => {
                    socket.get_mut().write_all(&bytes).await.unwrap();
                    let _ = tx.send(());
                }
                Script::Read(tx) => {
                    let mut line = String::new();
                    socket.read_line(&mut line).await.unwrap();
                    let _ = tx.send(line);
                }
                Script::Eof(tx) => {
                    drop(socket);
                    let _ = tx.send(());
                    return;
                }
            }
        }
    });
    Peer(tx)
}
async fn started(
    families: &[Family],
    options: SubscriptionOptions,
) -> (
    tempfile::TempDir,
    Subscription,
    umbriel_client::Observer,
    Peer,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let (subscription, observer) = Endpoint::new(path)
        .subscribe(families, options)
        .await
        .unwrap();
    let peer = accept(&listener).await;
    assert!(peer.read().await.contains("subscribe"));
    (dir, subscription, observer, peer)
}
async fn streaming(subscription: &Subscription) {
    let mut status = subscription.watch_status();
    loop {
        if status.borrow_and_update().health == Health::Streaming {
            return;
        }
        status.changed().await.unwrap();
    }
}
async fn frame(observer: &mut umbriel_client::Observer) -> Observation {
    loop {
        let item = observer.recv().await.unwrap();
        if matches!(item, Observation::Frame { .. }) {
            return item;
        }
    }
}

#[tokio::test]
async fn two_observers_one_stalled_does_not_block_commands_or_other_observer() {
    let options = SubscriptionOptions {
        observer_capacity: 2,
        ..Default::default()
    };
    let (_dir, subscription, mut fast, peer) = started(&[Family::Windows], options).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut fast).await;
    let mut slow = subscription.observe().await.unwrap();
    for _ in 0..5 {
        peer.write(b"{\"data\":[],\"event\":\"windows\"}\n").await;
        assert!(matches!(
            frame(&mut fast).await.message(),
            Some(Message::Event { .. })
        ));
    }
    let handle = subscription.clone();
    let request = tokio::spawn(async move {
        handle
            .request(1, Request::Query(Query::Submap), Duration::from_secs(10))
            .await
    });
    assert!(peer.read().await.contains("submap"));
    peer.write(b"{\"ok\":\"reply\"}\n").await;
    assert_eq!(
        request.await.unwrap().unwrap().decode::<String>().unwrap(),
        "reply"
    );
    assert!(matches!(
        slow.recv().await.unwrap(),
        Observation::Epoch { .. }
    ));
    assert!(matches!(
        slow.recv().await.unwrap(),
        Observation::Frame { .. }
    ));
    assert!(matches!(
        slow.recv().await,
        Err(DeliveryEnd::Lagged { epoch: 1, .. })
    ));
    assert!(matches!(slow.recv().await, Err(DeliveryEnd::Lagged { .. })));
    subscription.shutdown().await;
}

#[tokio::test]
async fn cancellation_late_reply_cannot_complete_later_command() {
    let (_dir, subscription, mut observer, peer) =
        started(&[Family::Windows], Default::default()).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut observer).await;
    let handle = subscription.clone();
    let first = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("first"), Duration::from_secs(60))
            .await
    });
    assert!(peer.read().await.contains("first"));
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let handle = subscription.clone();
    let second = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("second"), Duration::from_secs(60))
            .await
    });
    peer.write(b"{\"event\":\"future_family\",\"data\":null}\n{\"event\":\"theme\",\"data\":null}\n{\"event\":\"windows\",\"data\":[]}\n{\"ok\":\"first-reply\"}\n").await;
    assert!(peer.read().await.contains("second"));
    peer.write(b"{\"ok\":\"second-reply\"}\n").await;
    assert_eq!(
        second.await.unwrap().unwrap().decode::<String>().unwrap(),
        "second-reply"
    );
    subscription.shutdown().await;
}

#[tokio::test]
async fn keyboard_blank_and_independent_initialization() {
    let (_dir, subscription, mut observer, peer) =
        started(&[Family::KeyboardLayout], Default::default()).await;
    peer.write(b"\n").await;
    assert!(matches!(
        frame(&mut observer).await.message(),
        Some(Message::Blank)
    ));
    streaming(&subscription).await;
    assert_eq!(
        subscription.status().family_status(Family::KeyboardLayout),
        FamilyStatus::Unseen
    );
    subscription.shutdown().await;

    let (_dir, subscription, mut observer, peer) = started(
        &[Family::Windows, Family::KeyboardLayout],
        Default::default(),
    )
    .await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut observer).await;
    streaming(&subscription).await;
    assert_eq!(
        subscription.status().family_status(Family::Windows),
        FamilyStatus::Observed
    );
    assert_eq!(
        subscription.status().family_status(Family::KeyboardLayout),
        FamilyStatus::Unseen
    );
    subscription.shutdown().await;
}

#[tokio::test]
async fn full_queue_retains_source_terminal_and_shutdown_closes_socket() {
    let (_dir, subscription, mut observer, peer) = started(
        &[Family::Windows],
        SubscriptionOptions {
            observer_capacity: 2,
            ..Default::default()
        },
    )
    .await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    streaming(&subscription).await;
    subscription.shutdown().await;
    assert_eq!(subscription.status().health, Health::Closed);
    assert_eq!(peer.read().await, "");
    assert!(matches!(
        observer.recv().await.unwrap(),
        Observation::Epoch { .. }
    ));
    assert!(matches!(
        observer.recv().await.unwrap(),
        Observation::Frame { .. }
    ));
    assert!(
        matches!(observer.recv().await, Err(DeliveryEnd::Source(terminal)) if terminal.reason == EndReason::LocalClose)
    );
}

#[tokio::test]
async fn last_owner_drop_closes_source_even_when_observer_survives() {
    let (_dir, subscription, mut observer, peer) =
        started(&[Family::Windows], Default::default()).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut observer).await;
    let clone = subscription.clone();
    drop(subscription);
    drop(clone);
    assert_eq!(peer.read().await, "");
    loop {
        if let Err(end) = observer.recv().await {
            assert!(
                matches!(end, DeliveryEnd::Source(terminal) if terminal.reason == EndReason::LocalClose)
            );
            break;
        }
    }
}

#[tokio::test]
async fn default_disconnect_is_terminal_and_truncated_tail_is_protocol_loss() {
    let (_dir, subscription, mut observer, peer) =
        started(&[Family::Windows], Default::default()).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n{\"event\":")
        .await;
    peer.eof().await;
    frame(&mut observer).await;
    loop {
        if let Err(end) = observer.recv().await {
            assert!(
                matches!(end, DeliveryEnd::Source(terminal) if matches!(terminal.reason, EndReason::Protocol(_)))
            );
            break;
        }
    }
    assert_eq!(subscription.status().epoch, 1);
    subscription.shutdown().await;
}

#[tokio::test]
async fn opt_in_reconnect_has_gap_new_epoch_and_never_replays_commands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let options = SubscriptionOptions {
        reconnect: Reconnect::Retry {
            delay: Duration::ZERO,
            max_attempts: 1,
        },
        ..Default::default()
    };
    let (subscription, mut observer) = Endpoint::new(path)
        .subscribe(&[Family::Windows, Family::KeyboardLayout], options)
        .await
        .unwrap();
    let first_peer = accept(&listener).await;
    assert!(first_peer.read().await.contains("subscribe"));
    first_peer
        .write(b"{\"event\":\"windows\",\"data\":[]}\n")
        .await;
    frame(&mut observer).await;
    let handle = subscription.clone();
    let active = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("must-not-replay"), Duration::from_secs(60))
            .await
    });
    assert!(first_peer.read().await.contains("must-not-replay"));
    let handle = subscription.clone();
    let queued = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("old-queued"), Duration::from_secs(60))
            .await
    });
    first_peer.eof().await;
    assert!(
        matches!(active.await.unwrap(), Err(Error::Ended { terminal, .. }) if terminal.unresolved.unwrap().possibly_sent)
    );
    let second_peer = accept(&listener).await;
    assert!(second_peer.read().await.contains("subscribe"));
    assert!(matches!(
        queued.await.unwrap(),
        Err(Error::Closed | Error::StaleEpoch { .. })
    ));
    assert_eq!(subscription.status().epoch, 2);
    assert_eq!(
        subscription.status().family_status(Family::Windows),
        FamilyStatus::Unseen
    );
    assert!(matches!(
        observer.recv().await.unwrap(),
        Observation::Gap { epoch: 1, .. }
    ));
    assert!(matches!(
        observer.recv().await.unwrap(),
        Observation::Epoch { epoch: 2 }
    ));
    second_peer
        .write(b"{\"event\":\"windows\",\"data\":[]}\n")
        .await;
    assert!(matches!(
        frame(&mut observer).await,
        Observation::Frame { epoch: 2, .. }
    ));
    assert!(matches!(
        subscription
            .request(1, Request::Msg("stale"), Duration::from_secs(60))
            .await,
        Err(Error::StaleEpoch {
            requested: 1,
            current: 2
        })
    ));
    let handle = subscription.clone();
    let fresh = tokio::spawn(async move {
        handle
            .request(2, Request::Msg("fresh"), Duration::from_secs(60))
            .await
    });
    assert!(second_peer.read().await.contains("fresh"));
    second_peer.write(b"{\"ok\":null}\n").await;
    fresh.await.unwrap().unwrap().decode::<()>().unwrap();
    subscription.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn queue_deadline_is_unsent_and_active_deadline_closes_with_uncertainty() {
    let (_dir, subscription, mut observer, peer) =
        started(&[Family::Windows], Default::default()).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut observer).await;
    let handle = subscription.clone();
    let active = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("active"), Duration::from_secs(60))
            .await
    });
    assert!(peer.read().await.contains("active"));
    let result = subscription
        .request(1, Request::Msg("queued"), Duration::from_secs(1))
        .await;
    assert!(matches!(
        result,
        Err(Error::TimedOut {
            possibly_sent: false,
            ..
        })
    ));
    assert!(matches!(
        active.await.unwrap(),
        Err(Error::TimedOut {
            possibly_sent: true,
            ..
        })
    ));
    assert_eq!(peer.read().await, "");
    subscription.shutdown().await;
}

#[tokio::test]
async fn command_ingress_and_observer_registration_are_bounded() {
    let options = SubscriptionOptions {
        command_capacity: 1,
        max_observers: 1,
        ..Default::default()
    };
    let (_dir, subscription, mut observer, peer) = started(&[Family::Windows], options).await;
    peer.write(b"{\"event\":\"windows\",\"data\":[]}\n").await;
    frame(&mut observer).await;
    assert!(matches!(
        subscription.observe().await,
        Err(Error::QueueFull)
    ));
    let handle = subscription.clone();
    let active = tokio::spawn(async move {
        handle
            .request(1, Request::Msg("active"), Duration::from_secs(60))
            .await
    });
    peer.read().await;
    assert!(matches!(
        subscription
            .request(1, Request::Msg("queued"), Duration::from_secs(1))
            .await,
        Err(Error::QueueFull)
    ));
    drop(observer);
    let _replacement = subscription.observe().await.unwrap();
    subscription.shutdown().await;
    assert!(matches!(active.await.unwrap(), Err(Error::Ended { .. })));
}

#[tokio::test]
async fn invalid_subscription_options_fail_before_connect() {
    let options = SubscriptionOptions {
        command_capacity: 0,
        ..Default::default()
    };
    assert!(matches!(
        Endpoint::new("/not-a-socket")
            .subscribe(&[Family::Windows], options)
            .await,
        Err(Error::InvalidOptions(_))
    ));
}
