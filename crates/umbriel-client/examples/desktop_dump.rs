//! Read-only desktop capture using one-shot queries and initial event snapshots.
#[path = "../src/desktop_dump/mod.rs"]
mod desktop_dump;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    desktop_dump::run().await
}
