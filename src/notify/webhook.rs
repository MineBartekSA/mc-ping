use std::{
    error::Error,
    fmt::{
        Debug,
        Display
    },
};
use futures_util::{future::BoxFuture, FutureExt};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use once_cell::sync::OnceCell;
use async_std::sync::Arc;

use crate::models::Status;
use super::{
    ApplyStatus,
    NotifyService,
    default_players_separator,
    read_config,
    try_request,
};

#[derive(Debug, Deserialize)]
struct WebhookConfig<D: ApplyStatus + Default + Debug + Send + Sync + Clone> {
    // Webhook URL
    webhook: String,

    // Webhook request body when there are players online
    #[serde(default = "D::default")]
    message: D,

    // Webhook request body when there aren't any players online
    empty_message: Option<D>,

    // Player list reparator. Defaults to "\n"
    #[serde(default = "default_players_separator")]
    players_separator: String,
}

#[derive(Debug)]
pub struct WebhookService<D: ApplyStatus + DeserializeOwned + Serialize + Default + Debug + Send + Sync + Clone> {
    name: &'static str,
    config: OnceCell<WebhookConfig<D>>,
}

impl<D: ApplyStatus + DeserializeOwned + Serialize + Default + Debug + Send + Sync + Clone> WebhookService<D> {
    pub const fn new(name: &'static str) -> Self {
        WebhookService { name, config: OnceCell::new() }
    }
}

impl<D: ApplyStatus + DeserializeOwned + Serialize + Default + Debug + Send + Sync + Clone> NotifyService for WebhookService<D> {
    fn init(&self) -> Result<(), Box<dyn Error>> {
        self.config.set(read_config(&format!("{}.json", self.name.to_lowercase()))?).unwrap();
        log::info!("initialized {}", self.name);
        Ok(())
    }

    fn notify(&self, status: Arc<Status>) -> BoxFuture<Result<(), Box<dyn Error>>>{
        async move {
            let config = self.config.get().unwrap();
            let message = if status.players.online == 0 {
                if let Some(empty) = config.empty_message.as_ref() {
                    empty
                } else {
                    &config.message
                }
            } else {
                &config.message
            };
            let mut prepared_message = message.clone();
            prepared_message.apply_status(status, &config.players_separator);
            let request = surf::post(&self.config.get().unwrap().webhook)
                .header("Content-Type", "application/json")
                .build();
            let body = serde_json::to_vec(&prepared_message)?;
            try_request(request, body, 0).await?;
            Ok(())
        }.boxed()
    }
}

impl<D: ApplyStatus + DeserializeOwned + Serialize + Default + Debug + Send + Sync + Clone> Display for WebhookService<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
