//! Synthetic by default. `windows --stdin` or `workspaces --stdin` reads one
//! complete JSON array per LF-delimited input line (not one row per line).
//! `--wire` instead consumes subscription event envelopes from stdin.
use std::{
    error::Error,
    io::{self, BufRead},
};

use umbriel_arrow::{windows, workspaces};
use umbriel_polars::{QueryResult, SnapshotFrame, polars::prelude::col};
use umbriel_protocol::{
    codec::decode,
    envelope::Message,
    model::{Event, EventDecode, Window, Workspace, decode_event},
};

#[derive(Debug)]
struct LocalObservation {
    connection_epoch: u64,
    receive_position: u64,
}

fn query(
    family: &str,
    bytes: &[u8],
    provenance: LocalObservation,
) -> Result<QueryResult<LocalObservation>, Box<dyn Error>> {
    let event = match family {
        "windows" => {
            let rows: Vec<Window> = decode(bytes)?; // Sonic through protocol.
            Event::Windows(rows)
        }
        "workspaces" => {
            let rows: Vec<Workspace> = decode(bytes)?;
            Event::Workspaces(rows)
        }
        _ => return Err("expected windows or workspaces".into()),
    };
    query_event(event, provenance)
}

fn query_event(
    event: Event,
    provenance: LocalObservation,
) -> Result<QueryResult<LocalObservation>, Box<dyn Error>> {
    let plan = match event {
        Event::Windows(rows) => {
            SnapshotFrame::from(windows(&rows, provenance)?)
                .lazy()
                .map(|plan| {
                    plan.filter(col("active")).select([
                        col("id"),
                        col("title"),
                        col("pid"),
                        col("x"),
                    ])
                })
        }
        Event::Workspaces(rows) => SnapshotFrame::from(workspaces(&rows, provenance)?)
            .lazy()
            .map(|plan| {
                plan.filter(col("active"))
                    .select([col("id"), col("name"), col("layout")])
            }),
        _ => return Err("expected windows or workspaces event".into()),
    };
    // Rows have already dropped. No socket owner or live compositor involved.
    Ok(plan.collect()?)
}

fn wire_query(
    family: &str,
    bytes: &[u8],
    provenance: LocalObservation,
) -> Result<Option<QueryResult<LocalObservation>>, Box<dyn Error>> {
    let position = provenance.receive_position;
    let malformed = |error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("malformed wire input at local position {position}: {error}"),
        )
    };
    let message = Message::decode(bytes).map_err(malformed)?;
    match &message {
        Message::Blank => {
            eprintln!("skipping blank subscription frame at local position {position}");
            Ok(None)
        }
        Message::Ok(_) => {
            eprintln!("skipping uncorrelated ok reply at local position {position}");
            Ok(None)
        }
        Message::Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("remote err at local position {position}: {error}"),
        )
        .into()),
        Message::Event { name, .. } if name.as_str() != family => Ok(None),
        Message::Event { .. } => match decode_event(&message).map_err(malformed)? {
            EventDecode::Known(event) => Ok(Some(query_event(event, provenance)?)),
            _ => Err("selected event family is unsupported".into()),
        },
    }
}

fn show(output: QueryResult<LocalObservation>) {
    let (source, batches) = output.into_parts();
    println!(
        "Derived from {:?}, local epoch {}, position {}",
        source.family, source.provenance.connection_epoch, source.provenance.receive_position
    );
    for batch in batches {
        println!(
            "native batch: {} rows, {} columns; {:?}",
            batch.height(),
            batch.width(),
            batch
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let family = args.first().map(String::as_str).unwrap_or("windows");
    if !matches!(family, "windows" | "workspaces") {
        return Err("expected windows or workspaces".into());
    }
    let wire = args.get(1).is_some_and(|value| value == "--wire");
    if wire || args.get(1).is_some_and(|value| value == "--stdin") {
        for (position, line) in io::stdin().lock().lines().enumerate() {
            let line = line?;
            let provenance = LocalObservation {
                connection_epoch: 0,
                receive_position: u64::try_from(position)?,
            };
            if wire {
                if let Some(output) = wire_query(family, line.as_bytes(), provenance)? {
                    show(output);
                }
            } else {
                show(query(family, line.as_bytes(), provenance)?);
            }
        }
    } else {
        let bytes: &[u8] = match family {
            "windows" => include_bytes!("data/windows.json"),
            "workspaces" => include_bytes!("data/workspaces.json"),
            _ => return Err("expected windows or workspaces".into()),
        };
        show(query(
            family,
            bytes,
            LocalObservation {
                connection_epoch: 0,
                receive_position: 0,
            },
        )?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_wire_preserves_physical_positions_and_empty_replacements() {
        let outputs: Vec<_> = include_str!("data/mixed-wire.ndjson")
            .lines()
            .enumerate()
            .filter_map(|(position, line)| {
                wire_query(
                    "windows",
                    line.as_bytes(),
                    LocalObservation {
                        connection_epoch: 0,
                        receive_position: position as u64,
                    },
                )
                .unwrap()
            })
            .collect();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].source().provenance.receive_position, 2);
        assert_eq!(outputs[0].batches()[0].height(), 1);
        assert_eq!(outputs[1].source().provenance.receive_position, 3);
        assert_eq!(outputs[1].batches()[0].height(), 0);
        assert_eq!(outputs[1].batches()[0].width(), 4);
    }

    #[test]
    fn wire_failures_are_explicit() {
        for (bytes, expected) in [
            (br#"{"err":"synthetic rejection"}"#.as_slice(), "remote err"),
            (
                br#"{"event":"windows","data":[{}]}"#.as_slice(),
                "malformed wire input",
            ),
            (b"not JSON".as_slice(), "malformed wire input"),
            (b" ".as_slice(), "malformed wire input"),
        ] {
            let error = wire_query(
                "windows",
                bytes,
                LocalObservation {
                    connection_epoch: 0,
                    receive_position: 8,
                },
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains(expected), "{error}");
            assert!(error.contains("position 8"), "{error}");
        }
    }
}
