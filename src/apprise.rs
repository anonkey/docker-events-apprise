use anyhow::{anyhow, Result};
use reqwest::{header::ACCEPT, Client};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyMessageType {
    Info,
    Success,
    Warning,
    Failure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyBodyType {
    Text,
    Markdown,
    Html,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotifyPayload {
    // Config key to use in /notify/{KEY}
    #[serde(skip_serializing)]
    pub key: String,
    // Body as html, md or plain text
    pub body: String,
    // Notification title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // Message type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<NotifyMessageType>,
    // Targeted tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    // Body type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<NotifyBodyType>,
}

#[derive(Clone, Debug)]
pub struct AppriseClient {
    client: Client,
    notify_url: Url,
}

const NOTIFY_URL: &str = "/notify/";

impl AppriseClient {
    pub fn new(api_url: String) -> Result<Self> {
        let base_url = Url::parse(&api_url)?;

        Ok(AppriseClient {
            client: Client::new(),
            notify_url: base_url.join(NOTIFY_URL)?,
        })
    }

    pub async fn notify(&self, payload: NotifyPayload) -> Result<()> {
        let key = payload.key.to_owned();
        let body = serde_json::to_value(payload)?;

        let response = self
            .client
            .post(self.notify_url.join(&key)?)
            .header(ACCEPT, "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if status == 200 {
            Ok(())
        } else {
            Err(anyhow!(format!("Wrong status code {status}")))
        }
    }
}
