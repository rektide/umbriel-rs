use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    process::Command,
    time::timeout,
};

fn frame(title: &str) -> String {
    format!(
        r#"{{"event":"windows","data":[{{"id":"A","workspace":"ws","scratchpad":"","active":true,"app_id":"app","title":"{title}","xdg_tag":"","content_type":"none","floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}]}}
"#
    )
}

#[tokio::test]
async fn sigterm_during_pending_dwell_closes_socket_without_accepting() {
    timeout(Duration::from_secs(5), async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args(["--socket", path.to_str().unwrap(), "--dwell-ms", "60000", "--shutdown-timeout-ms", "500"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let (read, mut write) = socket.into_split();
        let mut read = BufReader::new(read);
        let mut subscription = String::new();
        read.read_line(&mut subscription).await.unwrap();
        write.write_all(frame("pending").as_bytes()).await.unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        let mut kinds = Vec::new();
        loop {
            line.clear();
            assert!(stdout.read_line(&mut line).await.unwrap() > 0);
            let value: serde_json::Value = serde_json::from_str(&line).unwrap();
            let kind = value["fields"]["event"].as_str().unwrap().to_owned();
            kinds.push(kind.clone());
            if kind == "source.ready" {
                break;
            }
        }
        assert!(
            Command::new("kill")
                .args(["-TERM", &child.id().unwrap().to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        assert!(child.wait().await.unwrap().success());
        let mut tail = String::new();
        stdout.read_to_string(&mut tail).await.unwrap();
        for line in tail.lines() {
            kinds.push(
                serde_json::from_str::<serde_json::Value>(line).unwrap()["fields"]["event"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        assert_eq!(kinds, ["source.epoch", "source.ready", "source.stopping"]);
        let mut byte = [0];
        assert_eq!(read.read(&mut byte).await.unwrap(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn blocked_output_cannot_prevent_bounded_signal_exit() {
    timeout(Duration::from_secs(8), async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blocked.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args(["--socket", path.to_str().unwrap(), "--dwell-ms", "0", "--shutdown-timeout-ms", "100"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let (read, mut write) = socket.into_split();
        let mut read = BufReader::new(read);
        let mut line = String::new();
        read.read_line(&mut line).await.unwrap();
        // Leave stdout unread. This record exceeds the pipe capacity and blocks
        // the synchronous formatter thread, not the cancellation-aware runtime.
        write.write_all(frame(&"x".repeat(512 * 1024)).as_bytes()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            Command::new("kill")
                .args(["-TERM", &child.id().unwrap().to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        let status = timeout(Duration::from_secs(2), child.wait()).await.unwrap().unwrap();
        assert!(!status.success(), "bounded drain timeout is an explicit failure");
        let mut byte = [0];
        assert_eq!(read.read(&mut byte).await.unwrap(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn missing_initial_socket_fails_without_fabricated_ready() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
        .args(["--socket", dir.path().join("absent.sock").to_str().unwrap()])
        .output()
        .await
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[tokio::test]
async fn configured_reconnect_starts_fresh_dwell_then_exhausts_explicitly() {
    timeout(Duration::from_secs(8), async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reconnect.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args([
                "--socket",
                path.to_str().unwrap(),
                "--dwell-ms",
                "500",
                "--reconnect-attempts",
                "1",
                "--reconnect-delay-ms",
                "10",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut subscription = String::new();
        socket.read_line(&mut subscription).await.unwrap();
        socket.get_mut().write_all(frame("before loss").as_bytes()).await.unwrap();
        let mut records = Vec::new();
        loop {
            let record: serde_json::Value = serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            let ready = record["fields"]["event"] == "source.ready";
            records.push(record);
            if ready {
                break;
            }
        }
        drop(socket);
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        subscription.clear();
        socket.read_line(&mut subscription).await.unwrap();
        socket.get_mut().write_all(frame("after recovery").as_bytes()).await.unwrap();
        loop {
            let record: serde_json::Value = serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            let accepted = record["fields"]["event"] == "activity.accepted";
            records.push(record);
            if accepted {
                break;
            }
        }
        let accepted = &records.last().unwrap()["fields"];
        assert_eq!(accepted["epoch"], 2);
        assert_eq!(accepted["title"], "after recovery");
        assert!(accepted["event_mono_ms"].as_u64().unwrap() - accepted["since_mono_ms"].as_u64().unwrap() >= 500);
        assert_eq!(records.iter().filter(|r| r["fields"]["event"] == "activity.accepted").count(), 1);
        drop(listener);
        drop(socket);
        assert!(!child.wait().await.unwrap().success());
        while let Some(line) = lines.next_line().await.unwrap() {
            records.push(serde_json::from_str(&line).unwrap());
        }
        assert_eq!(records.iter().filter(|r| r["fields"]["event"] == "source.gap").count(), 2);
        assert_eq!(records.iter().filter(|r| r["fields"]["event"] == "source.epoch").count(), 2);
        assert_eq!(records.last().unwrap()["fields"]["event"], "source.stopping");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn zero_attempts_disables_transport_recovery() {
    timeout(Duration::from_secs(5), async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-reconnect.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args(["--socket", path.to_str().unwrap(), "--reconnect-attempts", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut subscription = String::new();
        socket.read_line(&mut subscription).await.unwrap();
        drop(socket); // Listener remains available: any retry would connect.
        let output = child.wait_with_output().await.unwrap();
        assert!(!output.status.success());
        let records: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.iter().filter(|r| r["fields"]["event"] == "source.epoch").count(), 1);
        assert!(
            !records
                .iter()
                .any(|r| r["fields"]["event"] == "source.ready" || r["fields"]["event"] == "activity.accepted")
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn human_view_exposes_title_and_same_window_metadata_without_another_dwell() {
    timeout(Duration::from_secs(5), async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("titles.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args(["--socket", path.to_str().unwrap(), "--dwell-ms", "30", "--output", "human"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        socket.read_line(&mut String::new()).await.unwrap();
        socket.get_mut().write_all(frame("synthetic initial title").as_bytes()).await.unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        loop {
            let line = lines.next_line().await.unwrap().unwrap();
            if line.contains("accepted") {
                assert!(line.contains("accepted \"synthetic initial title\" app=\"app\" window=\"A\" workspace=\"ws\""));
                break;
            }
        }
        socket.get_mut().write_all(frame("synthetic changed title").as_bytes()).await.unwrap();
        let line = lines.next_line().await.unwrap().unwrap();
        assert!(line.contains("metadata \"synthetic changed title\" app=\"app\" window=\"A\""));
        Command::new("kill").args(["-TERM", &child.id().unwrap().to_string()]).status().await.unwrap();
        assert!(child.wait().await.unwrap().success());
    })
    .await
    .unwrap();
}
