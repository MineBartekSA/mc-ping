use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
};
use futures_util::{future::BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use once_cell::sync::OnceCell;
use async_std::sync::Arc;
use serde_json::Value;

use crate::models::Status;
use super::{
    NotifyService,
    default_players_separator,
    try_request,
    read_config,
};

#[derive(Debug, Deserialize)]
struct CustomConfig {
    // Custom HTTP URL where upon a status chnage a POST request will be sent
    url: String,

    // Headers for status change requests
    headers: Option<HashMap<String, String>>,

    // Custom data added to the status change request
    custom_data: Option<HashMap<String, Value>>,

    // Player list reparator. Defaults to "\n"
    #[serde(default = "default_players_separator")]
    players_separator: String,
}

static CONFIG: OnceCell<CustomConfig> = OnceCell::new();

pub const INSTANCE: Custom = Custom{};

#[derive(Debug)]
pub struct Custom;

#[derive(Serialize)]
struct StatusWithCustomData<'a> {
    status: &'a Status,
    custom_data: HashMap<&'a String, Value>,
}

impl NotifyService for Custom {
    fn init(&self) -> Result<(), Box<dyn Error>> {
        CONFIG.set(read_config("custom.json")?).unwrap();
        log::info!("initialized");
        Ok(())
    }
    
    fn notify(&self, status: Arc<Status>) -> BoxFuture<Result<(), Box<dyn Error>>> {
        async move {
            let config = CONFIG.get().unwrap();
            let mut build = surf::post(&config.url).header("Content-Type", "application/json");
            if let Some(headers) = config.headers.as_ref() {
                for (header, value) in headers {
                    build = build.header(header.as_str(), status.format(value, &config.players_separator));
                }
            }
            let req = build.build();
            let body = if let Some(data) = config.custom_data.as_ref() {
                let mut custom_data = HashMap::new();
                for (key, value) in data {
                    if let Value::String(inner_value) = value {
                        custom_data.insert(key, Value::String(status.format(inner_value, &config.players_separator)));
                    } else {
                        custom_data.insert(key, value.clone());
                    }
                }
                serde_json::to_vec(&StatusWithCustomData{status: status.as_ref(), custom_data})?
            } else {
                serde_json::to_vec(status.as_ref())?
            };
            try_request(req, body, 0).await?;
            Ok(())
        }.boxed()
    }
}

impl Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
