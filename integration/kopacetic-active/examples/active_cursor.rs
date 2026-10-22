use std::{error::Error, time::Duration};
use umbriel_client::{Discovery, Reconnect, SubscriptionOptions};
use umbriel_kopacetic_active::CursorFeed;
use umbriel_protocol::envelope::Family;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source_name = std::env::args().nth(1).ok_or("usage: active_cursor SOURCE-NAME")?;
    let endpoint = Discovery::from_env().resolve()?;
    let (source, observer) = endpoint
        .subscribe(
            &[Family::Windows],
            SubscriptionOptions {
                reconnect: Reconnect::Retry {
                    delay: Duration::from_secs(1),
                    max_attempts: 5,
                },
                ..SubscriptionOptions::default()
            },
        )
        .await?;
    let (feed, mut reader) = CursorFeed::channel(source_name)?;
    let mut task = tokio::spawn(feed.follow(observer));
    let result = loop {
        println!("{:#?}", reader.snapshot());
        tokio::select! {
            result = &mut task => break result.map(|end| eprintln!("{end}")),
            result = tokio::signal::ctrl_c() => {
                result?;
                task.abort();
                let _ = task.await;
                break Ok(());
            }
            result = reader.changed() => {
                if result.is_err() { break task.await.map(|end| eprintln!("{end}")); }
            }
        }
    };
    println!("{:#?}", reader.snapshot());
    source.shutdown().await;
    result?;
    Ok(())
}
