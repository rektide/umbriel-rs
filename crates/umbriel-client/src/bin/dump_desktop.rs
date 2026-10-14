//! A normal Cargo binary, sharing its implementation with the desktop example.
#[path = "../desktop_dump/mod.rs"]
mod desktop_dump;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    desktop_dump::run().await
}
