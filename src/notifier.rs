use crate::{
    apprise::{AppriseClient, NotifyMessageType},
    utils::TypeOrVector,
};
use anyhow::{Context, Ok, Result};
use bollard_next::service::EventMessage;
use serde::{Deserialize, Serialize};
use tokio::spawn;

use crate::{
    apprise::{NotifyBodyType, NotifyPayload},
    cli, matcher,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppriseEndpoint {
    // Config key to use in /notify/{KEY}
    pub key: String,
    // Apprise message type
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<NotifyMessageType>,
    // Apprise tag
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notifier {
    pub endpoint: AppriseEndpoint,
    pub matchers: Option<TypeOrVector<matcher::EventMatcher>>,
}

impl Notifier {
    pub fn match_event(&self, event: &EventMessage) -> bool {
        match &self.matchers {
            Some(matchers) => match matchers {
                TypeOrVector::Type(matcher) => matcher.match_event(event),
                TypeOrVector::Vector(vector) => {
                    vector.iter().any(|matcher| matcher.match_event(event))
                }
            },
            None => true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EventHandler {
    pub apprise_client: AppriseClient,
    pub docker_client: bollard_next::Docker,
    pub notifiers: Vec<Notifier>,
}

impl EventHandler {
    pub fn new(args: cli::Args) -> Result<EventHandler> {
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

async fn send_notify_request(
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

pub async fn notify(
    event_handler: &EventHandler,
    notifier: &Notifier,
    event: &EventMessage,
) -> Result<()> {
    let event = event.clone();
    let notifier = notifier.clone();
    let event_handler = event_handler.clone();
    spawn(async move {
        let result = send_notify_request(&event_handler, &notifier, event).await;
        let endpoint_key = notifier.endpoint.key;

        match result {
            std::result::Result::Ok(_) => println!("{} notified", endpoint_key),
            Err(err) => {
                eprintln!("Notify error while notifying {} : {:#?}", endpoint_key, err)
            }
        }
    })
    .await?;
    Ok(())
}
