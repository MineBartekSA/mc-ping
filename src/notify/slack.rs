use serde_json::{json, map::Map, Value};
use serde::{Serialize, Deserialize};
use async_std::sync::Arc;

use crate::models::Status;
use super::{
    WebhookService,
    ApplyStatus,
};

pub static INSTANCE: WebhookService<Message> = WebhookService::new("Slack");

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message(Map<String, Value>);

impl Default for Message {
    fn default() -> Self {
        let default = json!({
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "Status change: %online/%max\nPlayers:```%players```"
                    }
                }
            ]
        });
        Self(default.as_object().unwrap().to_owned())
    }
}

impl ApplyStatus for Message {
    fn apply_status<S: AsRef<str>>(&mut self, status: Arc<Status>, player_separator: &S) {
        self.0.apply_status(status, player_separator);
    }
}
