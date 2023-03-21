use std::{error::Error, fmt::Display};
use futures_util::{future::BoxFuture, FutureExt};
use serde_json::{json, map::Map, Value};
use serde::{Deserialize, Serialize};
use once_cell::sync::OnceCell;
use async_std::sync::Arc;

use crate::models::Status;
use super::{
    ApplyStatus,
    NotifyService,
    default_players_separator,
    try_request,
    read_config,
};

#[derive(Debug, Deserialize)]
struct FirebaseConfig {
    // Firebase Cloud Messaging Legacy API Server Key
    key: String,
    
    #[serde(flatten)]
    notification: Notification,

    // Player list reparator. Defaults to "\n"
    #[serde(default = "default_players_separator")]
    players_separator: String,
}

static CONFIG: OnceCell<FirebaseConfig> = OnceCell::new();

pub const INSTANCE: Firebase = Firebase{};

#[derive(Debug)]
pub struct Firebase;

impl NotifyService for Firebase {
    fn init(&self) -> Result<(), Box<dyn Error>> {
        CONFIG.set(read_config("firebase.json")?).unwrap();
        log::info!("initialized");
        Ok(())
    }
    
    fn notify(&self, status: Arc<Status>) -> BoxFuture<Result<(), Box<dyn Error>>> {
        async move {
            let config = CONFIG.get().unwrap();
            let mut notification = config.notification.clone();
            notification.apply_status(status, &config.players_separator);
            let req = surf::post("https://fcm.googleapis.com/fcm/send")
                .header("Authorization", format!("key={}", config.key))
                .header("Content-Type", "application/json")
                .build();
            let body = serde_json::to_vec(&notification)?;
            try_request(req, body, 0).await?;
            Ok(())
        }.boxed()
    }
}

impl Display for Firebase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Notification {
    // The recipiant of the notification
    to: Option<String>,

    // A list of recipiants of the notification
    registration_ids: Option<Vec<String>>,

    // The condition for determining targets of the notification
    condition: Option<String>,

    // An identifier for the messages that allows for grouping them
    collapse_key: Option<String>,

    // The priority of the notification
    priority: Option<String>,

    // The time (in seconds) to live on the Firebase servers
    tile_to_live: Option<u64>,

    // Notification custom data
    data: Option<Map<String, Value>>,

    // Notificaiton when there are players online
    notification: Map<String, Value>,

    // Notification when there aren't any players online
    #[serde(skip_serializing)]
    empty_notofication: Option<Map<String, Value>>,
}

impl ApplyStatus for Notification {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S) {
        if status.players.online == 0 {
            if let Some(empty) = self.empty_notofication.as_ref() {
                self.notification = empty.clone();
            }
        }
        if let Some(condition) = self.condition.as_mut() {
            *condition = status.format(&condition, player_separator);
        }
        if let Some(data) = self.data.as_mut() {
            data.apply_status(status.clone(), player_separator);
        }
        self.notification.apply_status(status, player_separator);
    }
}

impl Default for Notification {
    fn default() -> Self {
        let data = json!({
            "title": "Status change: %online/%max",
            "body": "Server: %host:%port\nPlayers:\n%players"
        });
        let empty = json!({
            "title": "Status change: %online/%max",
            "body": "Server: %host:%port",
        });
        Self{
            to: Some("/topics/all".to_owned()),
            registration_ids: None,
            condition: None,
            collapse_key: None,
            priority: None,
            tile_to_live: None,
            data: None,
            notification: data.as_object().unwrap().to_owned(),
            empty_notofication: Some(empty.as_object().unwrap().to_owned()),
        }
    }
}
