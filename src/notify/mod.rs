use std::{error::Error, fs, fmt::Display};
use futures_util::future::{BoxFuture, FutureExt};
use serde_json::{map::Map, Value};
use async_std::{sync::Arc, task};
use once_cell::sync::Lazy;

use crate::models::{Status, InternalError};

#[cfg(any(feature = "discord", feature = "slack"))]
mod webhook;
#[cfg(any(feature = "discord", feature = "slack"))]
use webhook::WebhookService;

#[cfg(feature = "firebase")]
mod firebase;
#[cfg(feature = "discord")]
mod discord;
#[cfg(feature = "slack")]
mod slack;
#[cfg(feature = "custom")]
mod custom;

static SERVICES: &[&(dyn NotifyService + Send + Sync)] = &[
    #[cfg(feature = "firebase")]
    &firebase::INSTANCE,
    #[cfg(feature = "discord")]
    &discord::INSTANCE,
    #[cfg(feature = "slack")]
    &slack::INSTANCE,
    #[cfg(feature = "custom")]
    &custom::INSTANCE,
];

trait NotifyService: Display {
    fn init(&self) -> Result<(), Box<dyn Error>>;
    fn notify(&self, status: Arc<Status>) -> BoxFuture<Result<(), Box<dyn Error>>>;
}

pub async fn init() -> Result<(), Box<dyn Error>> {
    for service in SERVICES {
        service.init()?;
    }
    Ok(())
}

pub fn notify(status: Status) {
    let status = Arc::new(status);
    for service in SERVICES {
        let copy = status.clone();
        task::spawn(async move {
            if let Err(err) = service.notify(copy).await {
                log::error!("failed to notify using {} service: {}", service, err)
            }
        });
    }
}

fn default_players_separator() -> String {
    "\n".to_owned()
}

fn read_config<T: serde::de::DeserializeOwned>(filename: &str) -> Result<T, Box<dyn Error>> {
    let filename = format!("./{}", filename);
    match fs::read(&filename) {
        Ok(file) => {
            let config = serde_json::from_slice::<T>(&file)?;
            Ok(config)
        }
        Err(err) => Err(InternalError::new(format!("unable to read {} file: {}", filename, err)).into()),
    }
}

const MAX_REQUESTS: u8 = 5;

static CLIENT: Lazy<surf::Client> = Lazy::new(|| {
    let client: surf::Client = surf::Config::new()
        .set_timeout(Some(std::time::Duration::from_secs(5)))
        .set_http_keep_alive(true)
        .try_into().unwrap();
    client.with(surf::middleware::Redirect::default())
        .with(surf::middleware::Logger::default())
});

fn try_request(request: surf::Request, json: Vec<u8>, attempt: u8) -> BoxFuture<'static, Result<(), Box<dyn Error>>> {
    async move {
        let mut fail = false;
        let mut copy = request.clone();
        copy.body_bytes(&json);
        match CLIENT.send(copy).await {
            Ok(mut res) => {
                if res.status() != surf::StatusCode::Ok && res.status() != surf::StatusCode::NoContent {
                    log::warn!("request failed. Status: {}\n{}", res.status(), res.body_string().await?);
                    fail = true;
                }
            }
            Err(err) => {
                log::error!("failed to send request: {}", err);
                fail = true;
            }
        }
        if fail {
            if attempt < MAX_REQUESTS {
                try_request(request, json, attempt + 1).await
            } else {
                Err(InternalError::new("request failed too many times").into())
            }
        } else {
            Ok(())
        }
    }.boxed()
}

pub trait ApplyStatus {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S);
}

impl ApplyStatus for Map<String, Value> {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S) {
        for (_, value) in self.iter_mut() {
            value.apply_status(status.clone(), player_separator);
        }
    }
}

impl ApplyStatus for Vec<Value> {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S) {
        for value in self.iter_mut() {
            value.apply_status(status.clone(), player_separator);
        }
    }
}

impl ApplyStatus for Value {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S) {
        match self {
            Value::String(text) => {
                *text = status.format(&text, player_separator);
            },
            Value::Array(array) => {
                array.apply_status(status, player_separator);
            },
            Value::Object(map) => {
                map.apply_status(status, player_separator);
            },
            _ => {},
        }
    }
}
