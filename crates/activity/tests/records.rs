use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::{Dispatch, subscriber::NoSubscriber};
use tracing_subscriber::{Layer, layer::SubscriberExt};
use umbriel_activity::{Payload, Record, Recorder, emit, output::Logfmt};
use umbriel_state::active::{ActiveWindow, StableActive};
use umbriel_tracking::activity::Event;

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Buffer {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
fn stable(title: &str) -> StableActive {
    StableActive {
        epoch: 2,
        since: Duration::from_secs(1),
        last_observed: Duration::from_secs(3),
        emitted_at: Duration::from_secs(6),
        window: Some(ActiveWindow {
            id: "A".into(),
            title: title.into(),
            app_id: "app".into(),
            workspace: "ws".into(),
            scratchpad: String::new(),
        }),
    }
}

#[test]
fn data_survives_disabled_host_diagnostics_and_preserves_scalar_contract() {
    let bytes = Buffer::default();
    let writer = bytes.clone();
    let dispatch = Dispatch::new(
        tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(move || writer.clone())
                .with_filter(tracing_subscriber::filter::filter_fn(|m| m.target() == "umbriel::activity")),
        ),
    );
    let mut recorder = Recorder::new(Duration::from_secs(5));
    let title = format!("line\n\"quoted\"\t\\{}", "long".repeat(2048));
    let accepted = recorder.record(Event::Accepted(stable(&title)), Duration::from_secs(6));
    let metadata = recorder.record(Event::Metadata(stable("updated")), Duration::from_secs(7));
    let mut none = stable("");
    none.window = None;
    let records = [
        accepted.clone(),
        metadata,
        recorder.record(Event::Accepted(none), Duration::from_secs(12)),
        recorder.record(
            Event::Gap {
                epoch: 2,
                reason: "bad\nframe".into(),
            },
            Duration::from_secs(13),
        ),
        recorder.record(Event::Epoch { epoch: 3 }, Duration::from_secs(14)),
        recorder.record(Event::Ready { epoch: 3 }, Duration::from_secs(14)),
    ];
    tracing::subscriber::with_default(NoSubscriber::default(), || {
        for record in &records {
            emit(record, &dispatch);
        }
    });
    let text = bytes.text();
    let values: Vec<serde_json::Value> = text.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(values.len(), 6);
    let fields = &values[0]["fields"];
    assert_eq!(fields["schema_version"], 1);
    assert_eq!(fields["title"], title);
    assert_eq!(fields["since_mono_ms"], 1000);
    assert_eq!(fields["dwell_ms"], 5000);
    assert_eq!(fields["event_time"], accepted.event_time);
    assert_eq!(values[1]["fields"]["accepted_seq"], 1);
    assert_eq!(values[2]["fields"]["state"], "none");
    assert!(values[2]["fields"].get("window_id").is_none());
    assert_eq!(values[3]["fields"]["event"], "source.gap");
    assert!(values[3]["fields"].get("state").is_none());
    assert_eq!(values[4]["fields"]["epoch"], 3);
    assert_eq!(values[5]["fields"]["event"], "source.ready");
    // Export uses original application identity/time, not a fresh recorder.
    emit(&accepted, &dispatch);
    let replay: serde_json::Value = serde_json::from_str(bytes.text().lines().last().unwrap()).unwrap();
    assert_eq!(replay["fields"], values[0]["fields"]);
}

#[test]
fn logfmt_uses_one_line_and_json_escaped_string_values() {
    let bytes = Buffer::default();
    let writer = bytes.clone();
    let dispatch =
        Dispatch::new(tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().event_format(Logfmt).with_writer(move || writer.clone())));
    let title = "equals= space \"quote\"\nline\ttab\\slash";
    let record = Record {
        run_id: "test".into(),
        seq: 1,
        event_time: "original-time".into(),
        event_mono_ms: 6,
        dwell_ms: 5,
        payload: Payload::Accepted(stable(title)),
    };
    emit(&record, &dispatch);
    let text = bytes.text();
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains(&format!("title={}", serde_json::to_string(title).unwrap())));
    assert!(text.contains("seq=1"));
    assert!(text.ends_with('\n'));
}

#[test]
fn source_terminal_is_not_reported_twice() {
    let mut recorder = Recorder::new(Duration::from_secs(5));
    let _ = recorder.record(
        Event::Gap {
            epoch: 1,
            reason: "EOF".into(),
        },
        Duration::ZERO,
    );
    assert!(recorder.terminal("EOF".into(), Duration::ZERO).is_none());
    let _ = recorder.record(Event::Epoch { epoch: 2 }, Duration::ZERO);
    assert!(recorder.terminal("lag".into(), Duration::ZERO).is_some());
}
