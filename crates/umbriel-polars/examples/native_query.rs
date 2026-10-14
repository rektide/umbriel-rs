//! Native client-to-Polars composition: discover and connect through
//! `umbriel-client` directly, subscribe to one family, and run one finite lazy
//! query per selected snapshot. The source owner (`Subscription`) is retained
//! for the whole run; the bounded observer is consumed independently. Blocking
//! Polars collection runs on a blocking worker, never on the socket driver,
//! and only one query worker is active at a time.
//!
//! Each query result is handed to a caller sink and dropped; the loop retains
//! no output history. Gaps and observer terminal loss end the run with
//! explicit errors; this example uses `Reconnect::Never` defaults, so a
//! transport loss is a visible gap, not a hidden retry. A finite
//! selected-snapshot count (`--count`, default one) bounds the work; there is
//! no task-per-event pipeline.
//!
//! Windows queries filter on seat-global `active`, not workspace-local
//! remembered `focused`; workspaces select the active workspace of each output
//! group. Provenance is the actual local connection epoch and receive position
//! of the observed frame, not a server transaction or resume cursor.
//!
//! ```sh
//! cargo run -p umbriel-polars --example native_query -- windows --count 2
//! cargo run -p umbriel-polars --example native_query -- workspaces --socket /path/to/peer
//! ```
use std::{error::Error, path::PathBuf};

use umbriel_client::{DeliveryEnd, Discovery, Observation, Observer, SubscriptionOptions};
use umbriel_polars::{QueryResult, SnapshotFrame, SnapshotQuery, polars::prelude::col};
use umbriel_protocol::{
    envelope::{Family, Message},
    model::{Event, EventDecode, Window, Workspace, decode_event},
};

/// Actual local observation coordinates attached to each snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalProvenance {
    epoch: u64,
    position: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedFamily {
    Windows,
    Workspaces,
}

impl SelectedFamily {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "windows" => Some(Self::Windows),
            "workspaces" => Some(Self::Workspaces),
            _ => None,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Workspaces => "workspaces",
        }
    }
    fn wire(self) -> Family {
        match self {
            Self::Windows => Family::Windows,
            Self::Workspaces => Family::Workspaces,
        }
    }
}

#[derive(Debug)]
enum SelectedEvent {
    Windows(Vec<Window>),
    Workspaces(Vec<Workspace>),
}

struct Args {
    family: SelectedFamily,
    count: u64,
    socket: Option<PathBuf>,
}

fn parse_args_from(words: &[String]) -> Result<Args, Box<dyn Error + Send + Sync>> {
    let mut args = Args {
        family: SelectedFamily::Windows,
        count: 1,
        socket: None,
    };
    let mut words = words.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "windows" | "workspaces" => {
                args.family = SelectedFamily::parse(word).expect("matched word");
            }
            "--count" => {
                let value = words.next().ok_or("--count requires a value")?.as_str();
                args.count = value
                    .parse()
                    .map_err(|_| format!("--count received invalid value {value:?}"))?;
                if args.count == 0 {
                    return Err("--count must be at least 1".into());
                }
            }
            "--socket" => {
                let value = words.next().ok_or("--socket requires a value")?;
                args.socket = Some(PathBuf::from(value));
            }
            other => return Err(format!("unexpected argument {other:?}").into()),
        }
    }
    Ok(args)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args_from(&words)?;
    let discovery = Discovery {
        explicit: args.socket.clone(),
        ..Discovery::from_env()
    };
    let endpoint = discovery.resolve()?;
    println!("umbriel endpoint: {}", endpoint.path().display());
    // Default options keep Reconnect::Never: transport loss is an explicit gap.
    let (source, mut observer) = endpoint
        .subscribe(&[args.family.wire()], SubscriptionOptions::default())
        .await?;
    let result = consume_snapshots(args.family, args.count, &mut observer, |output| {
        // Bounded streaming consumption: print the native result, then let it
        // drop; no output history is retained.
        print_result(&output);
    })
    .await;
    // The observer alone does not keep the source alive; shutdown covers both
    // success and error paths and waits for actual socket/task cleanup.
    source.shutdown().await;
    result?;
    Ok(())
}

/// Consume the bounded observer until `count` selected snapshots have been
/// queried, streaming each result to `on_result` instead of retaining it. The
/// loop awaits each blocking query before receiving the next observation, so
/// at most one query worker is ever active. Every frame is reported; gaps and
/// terminal delivery loss end the run with explicit errors instead of silent
/// skips or retries.
async fn consume_snapshots(
    family: SelectedFamily,
    count: u64,
    observer: &mut Observer,
    mut on_result: impl FnMut(QueryResult<LocalProvenance>),
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let mut processed: u64 = 0;
    while processed < count {
        let observation = observer.recv().await.map_err(delivery_loss)?;
        let Observation::Frame {
            epoch,
            position,
            bytes,
        } = observation
        else {
            report_non_frame(&observation);
            if matches!(observation, Observation::Gap { .. }) {
                // The Gap variant is reported by report_non_frame; with
                // Reconnect::Never there is no fresh Epoch to wait for.
                return Err("source gap ended this run before the requested snapshot count".into());
            }
            continue;
        };
        match select_snapshot(family, epoch, position, &bytes) {
            Ok(Some(event)) => {
                // Awaiting this blocking worker before the next receive keeps
                // exactly one query worker active at a time.
                let output = query_off_thread(event, LocalProvenance { epoch, position }).await?;
                on_result(output);
                processed += 1;
            }
            Ok(None) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(processed)
}

/// Epoch announcements and gaps are printed as continuity evidence. A gap
/// means the snapshot state in between is unknown.
fn report_non_frame(observation: &Observation) {
    match observation {
        Observation::Epoch { epoch } => {
            eprintln!("epoch {epoch}: fresh connection continuity (no replay of prior events)");
        }
        Observation::Gap { epoch, terminal } => {
            eprintln!("epoch {epoch}: source gap, snapshot state is now unknown: {terminal:?}");
        }
        Observation::Frame { .. } => unreachable!("frames are handled separately"),
    }
}

/// Terminal observer loss is explicit: lag means an unknown number of
/// observations was missed; source end means no further snapshots can arrive.
fn delivery_loss(end: DeliveryEnd) -> Box<dyn Error + Send + Sync> {
    end.to_string().into()
}

/// Decode one observed frame. Unrelated families and non-event envelopes are
/// skipped with a note (envelope validation still applies to them); remote
/// errors and malformed selected payloads are explicit failures.
fn select_snapshot(
    family: SelectedFamily,
    epoch: u64,
    position: u64,
    frame: &[u8],
) -> Result<Option<SelectedEvent>, Box<dyn Error + Send + Sync>> {
    let message = Message::decode(frame)
        .map_err(|error| format!("epoch {epoch} position {position}: malformed frame: {error}"))?;
    if let Message::Event { name, .. } = &message
        && name.as_str() != family.as_str()
    {
        // Unrelated payloads receive envelope validation only, not family model validation.
        eprintln!(
            "epoch {epoch} position {position}: skipping unrelated {} event",
            name.as_str()
        );
        return Ok(None);
    }
    let event = match decode_event(&message) {
        Ok(EventDecode::Known(event)) => event,
        Ok(EventDecode::Unsupported { name, .. }) => {
            eprintln!(
                "epoch {epoch} position {position}: skipping unsupported event family {name}"
            );
            return Ok(None);
        }
        Ok(EventDecode::NotEvent) => match &message {
            Message::Err(reason) => {
                return Err(format!(
                    "epoch {epoch} position {position}: remote error in subscription stream: {reason}"
                )
                .into());
            }
            Message::Blank | Message::Ok(_) => {
                eprintln!(
                    "epoch {epoch} position {position}: skipping non-event envelope ({} bytes)",
                    frame.len()
                );
                return Ok(None);
            }
            Message::Event { .. } => unreachable!("event envelopes decode above"),
        },
        Err(error) => {
            return Err(format!(
                "epoch {epoch} position {position}: malformed selected {} payload: {error}",
                family.as_str()
            )
            .into());
        }
    };
    let selected = match (family, event) {
        (SelectedFamily::Windows, Event::Windows(rows)) => SelectedEvent::Windows(rows),
        (SelectedFamily::Workspaces, Event::Workspaces(rows)) => SelectedEvent::Workspaces(rows),
        // A single-family subscription never reaches this; keep it explicit.
        (_, event) => {
            eprintln!(
                "epoch {epoch} position {position}: skipping {} event while selecting {}",
                event.family().as_str(),
                family.as_str()
            );
            return Ok(None);
        }
    };
    Ok(Some(selected))
}

/// The illustrative finite lazy query. Windows filter on seat-global `active`,
/// not workspace-local remembered `focused`; workspaces select the active
/// workspace of each output group.
fn plan(
    event: SelectedEvent,
    provenance: LocalProvenance,
) -> Result<SnapshotQuery<LocalProvenance>, Box<dyn Error + Send + Sync>> {
    let plan = match event {
        SelectedEvent::Windows(rows) => {
            SnapshotFrame::from(umbriel_arrow::windows(&rows, provenance)?)
                .lazy()
                .map(|plan| {
                    plan.filter(col("active")).select([
                        col("id"),
                        col("app_id"),
                        col("title"),
                        col("w"),
                        col("h"),
                    ])
                })
        }
        SelectedEvent::Workspaces(rows) => {
            SnapshotFrame::from(umbriel_arrow::workspaces(&rows, provenance)?)
                .lazy()
                .map(|plan| {
                    plan.filter(col("active")).select([
                        col("index"),
                        col("id"),
                        col("name"),
                        col("output"),
                        col("layout"),
                    ])
                })
        }
    };
    Ok(plan)
}

/// Run the blocking finite collection on a blocking worker. The socket driver
/// never executes Polars work; one worker per selected snapshot is bounded by
/// the requested count.
async fn query_off_thread(
    event: SelectedEvent,
    provenance: LocalProvenance,
) -> Result<QueryResult<LocalProvenance>, Box<dyn Error + Send + Sync>> {
    let plan = plan(event, provenance)?;
    let output = tokio::task::spawn_blocking(move || plan.collect())
        .await
        .map_err(|joined| format!("blocking query worker failed: {joined}"))?
        .map_err(|error| format!("lazy query execution failed: {error}"))?;
    Ok(output)
}

fn print_result(output: &QueryResult<LocalProvenance>) {
    let source = output.source();
    println!(
        "derived {:?} output (kind {:?}; its input was a complete family replacement snapshot, schema v{}) from local epoch {}, receive position {}",
        source.family,
        output.kind(),
        source.schema_version,
        source.provenance.epoch,
        source.provenance.position
    );
    for batch in output.batches() {
        println!(
            "  native batch: {} rows, {} columns; {batch:?}",
            batch.height(),
            batch.width()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };
    use umbriel_arrow::polars_arrow::array::Utf8ViewArray;

    fn windows_frame(entries: String) -> Vec<u8> {
        format!("{{\"event\":\"windows\",\"data\":[{entries}]}}\n").into_bytes()
    }

    fn window_json(id: &str, title: &str, active: bool, focused: bool) -> String {
        format!(
            r#"{{"id":"{id}","workspace":"out:1","scratchpad":"","active":{active},"app_id":"app","title":"{title}","xdg_tag":"","content_type":"none","floating":false,"focused":{focused},"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}"#
        )
    }

    fn workspace_json(index: i32, name: &str, active: bool) -> String {
        format!(
            r#"{{"id":"ws{index}","name":"{name}","named":true,"index":{index},"output":"DP-1","active":{active},"focused":{active},"occupied":true,"layout":"scrolling"}}"#
        )
    }

    fn string_column(output: &QueryResult<LocalProvenance>, column: usize) -> Vec<String> {
        let batch = &output.batches()[0];
        let values = batch.columns()[column]
            .as_any()
            .downcast_ref::<Utf8ViewArray>()
            .expect("utf8 view column");
        (0..values.len())
            .map(|index| values.value(index).to_owned())
            .collect()
    }

    #[test]
    fn windows_query_selects_seat_active_not_remembered_focused() {
        let frame = windows_frame(
            [
                window_json("w1", "active and focused", true, true),
                window_json("w2", "active without remembered focus", true, false),
                window_json(
                    "w3",
                    "remembered focus without seat activation",
                    false,
                    true,
                ),
                window_json("w4", "inactive", false, false),
            ]
            .join(","),
        );
        let event = select_snapshot(SelectedFamily::Windows, 3, 7, &frame)
            .unwrap()
            .expect("selected windows event");
        let provenance = LocalProvenance {
            epoch: 3,
            position: 7,
        };
        let output = plan(event, provenance)
            .unwrap()
            .collect()
            .expect("finite query");
        // Seat activation decides; workspace-local remembered focus does not.
        assert_eq!(string_column(&output, 0), ["w1", "w2"]);
        assert_eq!(output.source().provenance, provenance);
        assert_eq!(output.batches()[0].height(), 2);
    }

    #[test]
    fn workspaces_query_selects_active_per_output_group() {
        let frame = format!(
            "{{\"event\":\"workspaces\",\"data\":[{},{}]}}\n",
            workspace_json(1, "web", true),
            workspace_json(2, "code", false),
        );
        let event = select_snapshot(SelectedFamily::Workspaces, 1, 4, frame.as_bytes())
            .unwrap()
            .expect("selected workspaces event");
        let output = plan(
            event,
            LocalProvenance {
                epoch: 1,
                position: 4,
            },
        )
        .unwrap()
        .collect()
        .unwrap();
        assert_eq!(string_column(&output, 1), ["ws1"]);
        assert!(output.source().provenance.epoch == 1 && output.source().provenance.position == 4);
    }

    #[test]
    fn unrelated_frames_are_skipped_and_failures_are_explicit() {
        // Unrelated family: envelope validation only, no family model validation.
        let unrelated = b"{\"event\":\"submap\",\"data\":\"resize\"}\n";
        assert!(
            select_snapshot(SelectedFamily::Windows, 1, 1, unrelated)
                .unwrap()
                .is_none()
        );
        // Blank acceptance evidence is not a snapshot.
        assert!(
            select_snapshot(SelectedFamily::Windows, 1, 2, b"")
                .unwrap()
                .is_none()
        );
        // Remote error inside the subscription stream is an explicit failure.
        let error = select_snapshot(SelectedFamily::Windows, 1, 3, b"{\"err\":\"nope\"}\n")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("remote error") && error.contains("position 3"),
            "{error}"
        );
        // A malformed selected payload is an explicit local decode failure.
        let error = select_snapshot(
            SelectedFamily::Windows,
            2,
            5,
            b"{\"event\":\"windows\",\"data\":[{}]}\n",
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("malformed selected windows payload"),
            "{error}"
        );
        // A malformed frame is never a snapshot.
        let error = select_snapshot(SelectedFamily::Windows, 2, 6, b"not json\n")
            .unwrap_err()
            .to_string();
        assert!(error.contains("malformed frame"), "{error}");
    }

    #[test]
    fn argument_parsing_is_finite_and_explicit() {
        let args = parse_args_from(&[]).unwrap();
        assert_eq!(args.family, SelectedFamily::Windows);
        assert_eq!(args.count, 1);
        let words: Vec<String> = ["workspaces", "--count", "2", "--socket", "/tmp/peer.sock"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let args = parse_args_from(&words).unwrap();
        assert_eq!(args.family, SelectedFamily::Workspaces);
        assert_eq!(args.count, 2);
        assert_eq!(args.socket, Some(PathBuf::from("/tmp/peer.sock")));
        for words in [
            vec!["--count".to_string()],
            vec!["--count".to_string(), "0".to_string()],
            vec!["--count".to_string(), "two".to_string()],
            vec!["--socket".to_string()],
            vec!["layers".to_string()],
        ] {
            assert!(parse_args_from(&words).is_err(), "{words:?}");
        }
    }

    /// Scripted Unix peer: asserts the subscribe request, writes `frames`, then
    /// optionally keeps the connection open or closes it (EOF).
    async fn scripted_subscription(
        family: Family,
        frames: Vec<Vec<u8>>,
        close_after_frames: bool,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let expected = match family {
            Family::Windows => "{\"cmd\":\"subscribe\",\"events\":[\"windows\"]}\n",
            Family::Workspaces => "{\"cmd\":\"subscribe\",\"events\":[\"workspaces\"]}\n",
            _ => unreachable!("selected families only"),
        }
        .to_owned();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut request = String::new();
            stream.read_line(&mut request).await.unwrap();
            assert_eq!(request, expected);
            for frame in frames {
                stream.get_mut().write_all(&frame).await.unwrap();
            }
            if close_after_frames {
                // Dropping the stream ends the connection from the peer side.
            }
        });
        (dir, path)
    }

    #[tokio::test]
    async fn collects_selected_snapshots_with_local_provenance_then_stops() {
        let frames = vec![
            windows_frame(window_json("w1", "first", true, false)),
            b"{\"event\":\"submap\",\"data\":\"resize\"}\n".to_vec(),
            windows_frame(window_json("w2", "second", true, true)),
        ];
        let (_dir, path) = scripted_subscription(Family::Windows, frames, false).await;
        let (source, mut observer) = umbriel_client::Endpoint::new(&path)
            .subscribe(&[Family::Windows], SubscriptionOptions::default())
            .await
            .unwrap();
        // count=2: the run stops after two selected snapshots without draining
        // the source dry; the unrelated submap frame is skipped, not queried.
        // The sink deliberately collects through the streaming seam for the
        // assertions below; main prints and drops instead.
        let mut collected: Vec<QueryResult<LocalProvenance>> = Vec::new();
        let processed = consume_snapshots(SelectedFamily::Windows, 2, &mut observer, |output| {
            collected.push(output);
        })
        .await
        .unwrap();
        source.shutdown().await;
        assert_eq!(processed, 2);
        assert_eq!(collected.len(), 2);
        assert!(
            collected
                .iter()
                .all(|output| output.source().provenance.epoch == 1)
        );
        let positions: Vec<u64> = collected
            .iter()
            .map(|output| output.source().provenance.position)
            .collect();
        assert!(
            positions[0] < positions[1],
            "positions must increase: {positions:?}"
        );
        assert_eq!(string_column(&collected[0], 0), ["w1"]);
        assert_eq!(string_column(&collected[1], 0), ["w2"]);
    }

    #[tokio::test]
    async fn transport_loss_is_an_explicit_gap_not_silence() {
        let frames = vec![windows_frame(window_json("w1", "only", true, false))];
        let (_dir, path) = scripted_subscription(Family::Windows, frames, true).await;
        let (source, mut observer) = umbriel_client::Endpoint::new(&path)
            .subscribe(&[Family::Windows], SubscriptionOptions::default())
            .await
            .unwrap();
        // Two snapshots requested, one delivered, then the peer closes: the gap
        // must surface as an error, never as a quiet early return.
        let mut seen = 0u64;
        let error = consume_snapshots(SelectedFamily::Windows, 2, &mut observer, |_| seen += 1)
            .await
            .unwrap_err()
            .to_string();
        source.shutdown().await;
        assert_eq!(seen, 1, "the pre-gap snapshot was streamed to the sink");
        assert!(error.contains("source gap"), "{error}");
    }
}
