//! Opt-in read-only witnesses against an existing activity service's retained
//! records. Nothing here emits synthetic journal entries or changes the service.
//! Assertions intentionally do not print private window titles.
use std::{collections::BTreeMap, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::{sleep, timeout},
};

async fn run(args: &[&str]) -> std::process::Output {
    timeout(
        Duration::from_secs(15),
        Command::new(env!("CARGO_BIN_EXE_umbriel-activity")).args(args).kill_on_drop(true).output(),
    )
    .await
    .unwrap()
    .unwrap()
}
fn objects(bytes: &[u8]) -> Vec<serde_json::Value> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[tokio::test]
#[ignore = "read-only: requires retained records from umbriel-activity.service and journalctl"]
async fn retained_titles_roundtrip_to_dump_human_shared_json_and_exact_resume() {
    let dump = run(&["dump", "--tail", "100"]).await;
    assert!(dump.status.success(), "finite native dump failed");
    let entries = objects(&dump.stdout);
    assert!(!entries.is_empty(), "existing service history is required");
    let native = timeout(
        Duration::from_secs(10),
        Command::new("journalctl")
            .args([
                "--user",
                "--no-pager",
                "--quiet",
                "--all",
                "-o",
                "json",
                "-n",
                "250",
                "_SYSTEMD_USER_UNIT=umbriel-activity.service",
                "TARGET=umbriel::activity",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(native.status.success());
    let native: BTreeMap<_, _> = objects(&native.stdout)
        .into_iter()
        .map(|entry| (entry["__CURSOR"].as_str().unwrap().to_owned(), entry))
        .collect();
    let mut count = 0;
    for entry in &entries {
        if entry["fields"]["state"] != "active" {
            continue;
        }
        let original = &native[entry["delivery"]["cursor"]["opaque"].as_str().unwrap()];
        assert!(entry["fields"]["title"] == original["UMBRIEL_TITLE"], "native title bytes changed");
        assert!(entry["fields"]["app_id"] == original["UMBRIEL_APP_ID"], "native app bytes changed");
        assert!(
            entry["fields"]["event_time"] == original["UMBRIEL_EVENT_TIME"],
            "original application time changed"
        );
        count += 1;
    }
    assert!(count > 0, "active records required for title witness");
    let exported = run(&["history", "--tail", "100", "--output", "jsonl", "--show-cursor"]).await;
    assert!(exported.status.success());
    let shared = objects(&exported.stdout);
    let mut matches = 0;
    for entry in &shared {
        if let Some(prior) = entries
            .iter()
            .find(|p| p["fields"]["seq"] == entry["fields"]["seq"] && p["fields"]["run_id"] == entry["fields"]["run_id"])
        {
            if !entry["fields"]["title"].is_null() {
                assert!(entry["fields"]["title"] == prior["fields"]["title"], "shared tracing title changed");
                matches += 1;
            }
        }
    }
    assert!(matches > 0);
    let human = run(&["history", "--tail", "100"]).await;
    assert!(human.status.success());
    let text = std::str::from_utf8(&human.stdout).unwrap();
    // Compare only control-free titles here. Synthetic control-heavy titles are
    // covered without journal writes by records.rs and the scripted socket test.
    assert!(
        entries
            .iter()
            .filter_map(|e| e["fields"]["title"].as_str())
            .filter(|t| !t.chars().any(char::is_control))
            .any(|title| text.contains(&serde_json::to_string(title).unwrap())),
        "human view did not expose an original title"
    );
    assert!(text.contains(" app=") && text.contains(" window=") && text.contains(" workspace="));
    let token = std::str::from_utf8(&exported.stderr).unwrap().lines().last().unwrap();
    let resumed = run(&["history", "--after", token, "--output", "jsonl"]).await;
    assert!(resumed.status.success(), "exact native resume failed");
    for entry in objects(&resumed.stdout) {
        assert!(
            !shared
                .iter()
                .any(|p| p["fields"]["seq"] == entry["fields"]["seq"] && p["fields"]["run_id"] == entry["fields"]["run_id"]),
            "exact handoff duplicated a record"
        );
    }
    let budget = run(&["dump", "--max-entries", "1"]).await;
    assert!(
        !budget.status.success() && budget.stdout.is_empty(),
        "over-budget finite read must not publish partial output"
    );
}

async fn terminate(child: &mut tokio::process::Child) {
    assert!(
        Command::new("kill")
            .args(["-TERM", &child.id().unwrap().to_string()])
            .status()
            .await
            .unwrap()
            .success()
    );
}

#[tokio::test]
#[ignore = "read-only: requires native local journal access"]
async fn native_empty_feed_follow_cancels_without_compositor_or_journal_writes() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
        .args(["history", "--run", "00000000-0000-4000-8000-000000000000", "--follow"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    sleep(Duration::from_millis(200)).await;
    terminate(&mut child).await;
    let output = timeout(Duration::from_secs(4), child.wait_with_output()).await.unwrap().unwrap();
    assert!(output.status.success() && output.stdout.is_empty());
}

#[tokio::test]
#[ignore = "read-only: requires enough retained activity records to fill a stdout pipe"]
async fn native_follow_replay_and_blocked_stdout_have_bounded_signal_shutdown() {
    for format in ["human", "jsonl", "logfmt"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
            .args(["history", "--tail", "1", "--follow", "--output", format])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        assert!(timeout(Duration::from_secs(8), lines.next_line()).await.unwrap().unwrap().is_some());
        terminate(&mut child).await;
        assert!(timeout(Duration::from_secs(4), child.wait()).await.unwrap().unwrap().success());
    }
    let mut blocked = Command::new(env!("CARGO_BIN_EXE_umbriel-activity"))
        .args(["--shutdown-timeout-ms", "100", "history", "--all", "--follow", "--output", "jsonl"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    sleep(Duration::from_millis(500)).await;
    terminate(&mut blocked).await;
    // Do not drain stdout yet: cancellation must not depend on a reader.
    let status = timeout(Duration::from_secs(3), blocked.wait()).await.unwrap().unwrap();
    assert!(!status.success(), "full stdout must report its shutdown timeout");
    let output = blocked.wait_with_output().await.unwrap();
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Elapsed"),
        "expected bounded shutdown timeout, not an unrelated read error"
    );
}
