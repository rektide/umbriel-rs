//! Minimal consumer of the reusable queue-before-timer activity runtime.
use std::{error::Error, time::Duration};
use umbriel_client::{Discovery, SubscriptionOptions};
use umbriel_protocol::envelope::Family;
use umbriel_tracking::activity::{Activity, Event};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (source, mut observer) = Discovery::from_env()
        .resolve()?
        .subscribe(&[Family::Windows], SubscriptionOptions::default())
        .await?;
    let mut activity = Activity::new(Duration::from_secs(5));
    let result = async {
        loop {
            match activity.next(&mut observer).await? {
                Event::Accepted(stable) | Event::Metadata(stable) => match stable.window {
                    Some(window) => println!("{}\t{}\t{}", window.id, window.app_id, window.title),
                    None => println!("(no active window)"),
                },
                event => eprintln!("{event:?}"),
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn Error>>(())
    }
    .await;
    source.shutdown().await;
    result
}
