use anyhow::Result;
use clap::Parser;
use futures_util::stream::StreamExt;
use notifier::EventHandler;
use std::{thread::sleep, time::Duration};

mod apprise;
mod cli;
mod matcher;
mod notifier;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();

    let event_handler = EventHandler::new(args)?;

    loop {
        let mut event_stream = event_handler.docker_client.events::<String>(None);

        while let Some(event_result) = event_stream.next().await {
            let event = match event_result {
                std::result::Result::Ok(event) => event,
                Err(err) => {
                    eprintln!("Docker daemon error {err}");
                    break;
                }
            };

            for notifier in event_handler.notifiers.iter() {
                if notifier.match_event(&event) {
                    match notifier::notify(&event_handler, notifier, &event).await {
                        Err(err) => {
                            eprintln!("Error can't notify {} : {err:#?}", notifier.endpoint.key)
                        }
                        std::result::Result::Ok(_) => {
                            println!("{} notified", notifier.endpoint.key)
                        }
                    };
                }
            }
        }

        eprintln!("Docker daemon connection lost");
        sleep(Duration::new(2, 0));
    }
}
