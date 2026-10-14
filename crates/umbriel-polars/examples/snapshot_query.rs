//! Synthetic by default. `windows --stdin` or `workspaces --stdin` reads one
//! complete JSON array per LF-delimited input line (not one row per line).
use std::{
    error::Error,
    io::{self, BufRead},
};

use polars::prelude::col;
use umbriel_arrow::{windows, workspaces};
use umbriel_polars::{QueryResult, SnapshotFrame};
use umbriel_protocol::{
    codec::decode,
    model::{Window, Workspace},
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
    let plan = match family {
        "windows" => {
            let rows: Vec<Window> = decode(bytes)?; // Sonic through protocol.
            SnapshotFrame::from(windows(&rows, provenance)?)
                .lazy()
                .map(|plan| {
                    plan.filter(col("focused")).select([
                        col("id"),
                        col("title"),
                        col("pid"),
                        col("x"),
                    ])
                })
        }
        "workspaces" => {
            let rows: Vec<Workspace> = decode(bytes)?;
            SnapshotFrame::from(workspaces(&rows, provenance)?)
                .lazy()
                .map(|plan| {
                    plan.filter(col("active"))
                        .select([col("id"), col("name"), col("layout")])
                })
        }
        _ => return Err("expected windows or workspaces".into()),
    };
    // Rows have already dropped. No socket owner or live compositor involved.
    Ok(plan.collect()?)
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
    if args.get(1).is_some_and(|value| value == "--stdin") {
        for (position, line) in io::stdin().lock().lines().enumerate() {
            let line = line?;
            show(query(
                family,
                line.as_bytes(),
                LocalObservation {
                    connection_epoch: 0,
                    receive_position: u64::try_from(position)?,
                },
            )?);
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
