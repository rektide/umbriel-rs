//! Tool/example composition, not a new library API or a restore controller.
use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, Write},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use serde::{Serialize, de::DeserializeOwned};
use sonic_rs::Value;
use tokio::time::{Instant, timeout};
use umbriel_client::{Discovery, Endpoint, Observation, Options, SubscriptionOptions};
use umbriel_protocol::{
    codec,
    envelope::{EventName, Family, Message},
    model::{
        Color, KeyboardLayouts, Layers, Overview, Submap, Tearing, Theme, Windows, Workspaces,
    },
    request::{Query, Request},
};

type ToolResult<T> = Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(
    name = "umbriel-dump-desktop",
    version,
    about = "Dump available Umbriel desktop state as JSON (read-only)",
    long_about = "Dump all seven query payloads plus initial theme and overview events as JSON.\n\
                  Captures are not atomic and are not a complete restore recipe: output modes,\n\
                  layout trees/ratios, and application session contents are not exposed by IPC.\n\
                  Write stdout to a file to keep the capture. Nothing is changed on the desktop."
)]
struct Args {
    /// Override UMBRIEL_SOCKET / XDG_RUNTIME_DIR + WAYLAND_DISPLAY discovery.
    #[arg(long)]
    socket: Option<PathBuf>,
    /// Deadline for each connect, query, or initial event collection.
    #[arg(long, default_value_t = 10_000, value_parser = clap::value_parser!(u64).range(1..))]
    timeout_ms: u64,
}

#[derive(Serialize)]
struct DesktopDump {
    format: &'static str,
    schema_version: u32,
    producer_version: &'static str,
    /// A model baseline, not a version reported by the running compositor.
    protocol_model_revision: &'static str,
    source_socket: String,
    started_at_unix_ms: u128,
    finished_at_unix_ms: u128,
    elapsed_ms: u128,
    atomic: bool,
    queries: BTreeMap<&'static str, Capture>,
    events: BTreeMap<&'static str, Capture>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
struct Capture {
    /// Local monotonic offsets; not timestamps supplied by the compositor.
    started_elapsed_ms: u128,
    received_elapsed_ms: u128,
    #[serde(flatten)]
    result: Payload,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Payload {
    Ok { data: Value },
    Unavailable { reason: String },
}

const LIMITATIONS: &[&str] = &[
    "Separate queries and initial events are not an atomic compositor-wide snapshot; cross-family references may race.",
    "Windows are mapped toplevels only; IDs and PIDs identify this live session, not relaunchable applications.",
    "Window x/y are layout target positions; w/h are committed client content sizes, not necessarily target resize sizes.",
    "IPC does not expose output position, mode, scale, transform, or the complete output configuration.",
    "IPC does not expose layout trees, column membership/order, split ratios, master settings, scroll offsets, or stacking order.",
    "Layer surfaces have namespace/output/layer/mapped state but no geometry or exclusive-zone data.",
    "Launch commands, application documents/session contents, and full compositor configuration are not captured; this is not a restore program.",
];

pub async fn run() -> ToolResult<()> {
    let args = Args::parse();
    let endpoint = Discovery {
        explicit: args.socket,
        ..Discovery::from_env()
    }
    .resolve()?;
    let deadline = Duration::from_millis(args.timeout_ms);
    let options = Options {
        connect_timeout: deadline,
        request_timeout: deadline,
        ..Options::default()
    };
    let dump = capture(&endpoint, &options).await?;
    // Finish capture and serialization before publishing anything on stdout.
    let mut json = sonic_rs::to_vec_pretty(&dump)?;
    json.push(b'\n');
    io::stdout().lock().write_all(&json)?;
    Ok(())
}

fn unix_ms() -> ToolResult<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

async fn capture(endpoint: &Endpoint, options: &Options) -> ToolResult<DesktopDump> {
    let started_at_unix_ms = unix_ms()?;
    let origin = Instant::now();
    let mut queries = BTreeMap::new();
    // Sequential one-shot requests avoid depending on subscriber command reuse.
    // Keep raw JSON fields (including additions) after validating known models.
    for query in Query::ALL {
        let started_elapsed_ms = origin.elapsed().as_millis();
        let reply = endpoint.request(Request::Query(query), options).await?;
        let data = match query {
            Query::Windows => validated_reply::<Windows>(&reply),
            Query::Workspaces => validated_reply::<Workspaces>(&reply),
            Query::Submap => validated_reply::<Submap>(&reply),
            Query::Layers => validated_reply::<Layers>(&reply),
            Query::Color => validated_reply::<Color>(&reply),
            Query::Tearing => validated_reply::<Tearing>(&reply),
            Query::KeyboardLayouts => validated_reply::<KeyboardLayouts>(&reply),
        };
        let result = match data {
            Ok(data) => Payload::Ok { data },
            Err(umbriel_client::Error::Remote(reason))
                if query == Query::KeyboardLayouts && reason == "no keyboard" =>
            {
                Payload::Unavailable { reason }
            }
            Err(error) => return Err(error.into()),
        };
        queries.insert(
            query.as_str(),
            Capture {
                started_elapsed_ms,
                received_elapsed_ms: origin.elapsed().as_millis(),
                result,
            },
        );
    }
    let events = initial_events(endpoint, options, origin).await?;
    Ok(DesktopDump {
        format: "umbriel-desktop-dump",
        schema_version: 1,
        producer_version: env!("CARGO_PKG_VERSION"),
        protocol_model_revision: "7666af0a43560658e7b966befcbe47c101968e1c",
        source_socket: endpoint.path().to_string_lossy().into_owned(),
        started_at_unix_ms,
        finished_at_unix_ms: unix_ms()?,
        elapsed_ms: origin.elapsed().as_millis(),
        atomic: false,
        queries,
        events,
        limitations: LIMITATIONS,
    })
}

fn validated_reply<T: DeserializeOwned>(
    reply: &umbriel_client::RawReply,
) -> Result<Value, umbriel_client::Error> {
    reply.decode::<T>()?;
    reply.decode::<Value>()
}

async fn initial_events(
    endpoint: &Endpoint,
    options: &Options,
    origin: Instant,
) -> ToolResult<BTreeMap<&'static str, Capture>> {
    let started_elapsed_ms = origin.elapsed().as_millis();
    let (source, mut observer) = endpoint
        .subscribe(
            &[Family::Theme, Family::Overview],
            SubscriptionOptions {
                io: options.clone(),
                ..SubscriptionOptions::default()
            },
        )
        .await?;
    let result = timeout(options.request_timeout, async {
        let mut events = BTreeMap::new();
        while events.len() < 2 {
            match observer.recv().await? {
                Observation::Epoch { .. } => {}
                Observation::Gap { terminal, .. } => {
                    return Err(format!("source gap during desktop capture: {terminal:?}").into());
                }
                Observation::Frame { bytes, .. } => {
                    if let Message::Event {
                        name: EventName::Known(family @ (Family::Theme | Family::Overview)),
                        data,
                    } = Message::decode(&bytes)?
                    {
                        if events.contains_key(family.as_str()) {
                            continue;
                        }
                        let bytes = data.as_raw_str().as_bytes();
                        match family {
                            Family::Theme => {
                                codec::decode::<Theme>(bytes)?;
                            }
                            Family::Overview => {
                                codec::decode::<Overview>(bytes)?;
                            }
                            _ => unreachable!("only theme/overview matched"),
                        }
                        events.insert(
                            family.as_str(),
                            Capture {
                                started_elapsed_ms,
                                received_elapsed_ms: origin.elapsed().as_millis(),
                                result: Payload::Ok {
                                    data: codec::decode(bytes)?,
                                },
                            },
                        );
                    }
                }
            }
        }
        Ok(events)
    })
    .await;
    // Also shut down on a decode error, source loss, or missing-family deadline.
    source.shutdown().await;
    result.map_err(|_| "timed out waiting for initial theme and overview snapshots")?
}
