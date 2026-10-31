//! Opt-in native field/readback smoke; this is visibility evidence, not durability.
use std::{
    process::Command,
    time::{Duration, Instant},
};
use umbriel_activity::{
    Recorder, emit,
    output::{Output, dispatch},
};
use umbriel_state::active::{ActiveWindow, StableActive};
use umbriel_tracking::activity::Event;

#[test]
#[ignore = "writes synthetic activity records to the local user journal"]
fn native_journal_fields_preserve_multiline_long_titles_and_none() {
    let dispatch = dispatch(Output::Journal).unwrap();
    let mut recorder = Recorder::new(Duration::from_secs(5));
    let title = format!("native probe\n\"quoted\"\\{}", "long".repeat(2048));
    let mut active = StableActive {
        epoch: 9,
        since: Duration::ZERO,
        last_observed: Duration::from_secs(1),
        emitted_at: Duration::from_secs(5),
        window: Some(ActiveWindow {
            id: "synthetic-journal-probe".into(),
            app_id: "umbriel-activity-test".into(),
            title: title.clone(),
            workspace: "probe".into(),
            scratchpad: String::new(),
        }),
    };
    let accepted = recorder.record(Event::Accepted(active.clone()), Duration::from_secs(5));
    emit(&accepted, &dispatch);
    active.window.as_mut().unwrap().title = "metadata".into();
    emit(&recorder.record(Event::Metadata(active.clone()), Duration::from_secs(6)), &dispatch);
    active.window = None;
    emit(&recorder.record(Event::Accepted(active), Duration::from_secs(11)), &dispatch);
    emit(
        &recorder.record(
            Event::Gap {
                epoch: 9,
                reason: "synthetic gap".into(),
            },
            Duration::from_secs(12),
        ),
        &dispatch,
    );
    let start = Instant::now();
    let entries = loop {
        let output = Command::new("journalctl")
            .args([
                "--user",
                "--no-pager",
                "--all",
                "-o",
                "json",
                &format!("_PID={}", std::process::id()),
                &format!("UMBRIEL_RUN_ID={}", accepted.run_id),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let entries: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        if entries.len() >= 4 {
            break entries;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "native records not visible");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(entries.len(), 4, "one native journal entry per emitted fact");
    assert_eq!(entries[0]["UMBRIEL_EVENT"], "activity.accepted");
    assert_eq!(entries[0]["UMBRIEL_SCHEMA_VERSION"], "1");
    assert_eq!(entries[0]["UMBRIEL_TITLE"], title);
    assert_eq!(entries[0]["UMBRIEL_WINDOW_ID"], "synthetic-journal-probe");
    let message = entries[0]["MESSAGE"].as_str().unwrap();
    assert!(message.contains("native probe\\n\\\"quoted\\\""));
    assert!(message.contains("app=\"umbriel-activity-test\""));
    assert!(message.contains("window=\"synthetic-journal-probe\""));
    assert!(message.contains("workspace=\"probe\""));
    assert!(!message.contains('\n'), "summary must stay on one journal line");
    assert_eq!(entries[0]["UMBRIEL_EVENT_TIME"], accepted.event_time);
    assert_eq!(entries[0]["SYSLOG_IDENTIFIER"], "umbriel-activity");
    assert_eq!(entries[0]["PRIORITY"], "5");
    assert_eq!(entries[1]["UMBRIEL_ACCEPTED_SEQ"], "1");
    assert_eq!(entries[2]["UMBRIEL_STATE"], "none");
    assert_eq!(entries[2]["MESSAGE"], "accepted no active window");
    assert!(entries[2].get("UMBRIEL_WINDOW_ID").is_none());
    assert_eq!(entries[3]["UMBRIEL_EVENT"], "source.gap");
    assert!(entries[3].get("UMBRIEL_STATE").is_none());
}
