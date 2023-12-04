use anyhow::{Context, Ok, Result};
use apprise::{AppriseClient, NotifyMessageType};
use bollard_next::service::EventMessage;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::{thread::sleep, time::Duration};
use tokio::spawn;

use clap::Parser;

use crate::apprise::{NotifyBodyType, NotifyPayload};

mod apprise;
mod matcher;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Config path
    path: String,

    /// Api url
    api_url: String,

    /// Docker socket path
    #[clap(long, short, default_value = "unix:///var/run/docker.sock")]
    socket: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppriseEndpoint {
    // Config key to use in /notify/{KEY}
    key: String,
    // Apprise message type
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<NotifyMessageType>,
    // Apprise tag
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notifier {
    endpoint: AppriseEndpoint,
    matchers: Vec<matcher::EventMatcher>,
}

#[derive(Clone, Debug)]
pub struct EventHandler {
    apprise_client: AppriseClient,
    docker_client: bollard_next::Docker,
    notifiers: Vec<Notifier>,
}

impl EventHandler {
    fn new(args: Args) -> Result<EventHandler> {
        let docker_client = bollard_next::Docker::connect_with_unix(
            &args.socket,
            120,
            bollard_next::API_DEFAULT_VERSION,
        )?;
        let data = std::fs::read_to_string(args.path)?;
        let notifiers = serde_yaml::from_str::<Vec<Notifier>>(&data)?;

        Ok(EventHandler {
            apprise_client: AppriseClient::new(args.api_url)?,
            docker_client,
            notifiers,
        })
    }
}

async fn notify(
    event_handler: &EventHandler,
    notifier: &Notifier,
    event: EventMessage,
) -> anyhow::Result<()> {
    // println!("Match {notifier:#?} with {event:#?}");

    event_handler
        .apprise_client
        .notify(NotifyPayload {
            key: notifier.endpoint.key.to_owned(),
            body: serde_json::to_string(&event)?,
            title: Some(format!(
                "{} {}",
                event.typ.context("Missing event type")?,
                event.action.context("Missing event action")?
            )),
            r#type: notifier.endpoint.r#type.to_owned(),
            tag: notifier.endpoint.tag.to_owned(),
            format: Some(NotifyBodyType::Text),
        })
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut event_handler = EventHandler::new(args)?;

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
                if notifier
                    .matchers
                    .iter()
                    .any(|matcher| matcher.match_event(&event))
                {
                    let event = event.clone();
                    let notifier = notifier.clone();
                    let event_handler = event_handler.clone();
                    spawn(async move {
                        let result = notify(&event_handler, &notifier, event).await;
                        let endpoint_key = notifier.endpoint.key;

                        match result {
                            std::result::Result::Ok(_) => println!("{} notified", endpoint_key),
                            Err(err) => {
                                eprintln!(
                                    "Notify error while notifying {} : {:#?}",
                                    endpoint_key, err
                                )
                            }
                        }
                    })
                    .await?;
                }
            }
        }

        eprintln!("Docker daemon connection lost");
        sleep(Duration::new(2, 0));
    }
}
