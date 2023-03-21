use std::{error::Error, borrow::Cow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat", default = "bool::default")]
    pub enforces_secure_chat: bool,
    #[serde(rename = "previewsChat", default = "bool::default")]
    pub previews_chat: bool,
    // #[serde(rename = "forgeData")]
    // pub forge_data: Option<ForgeData>, // TODO: Implement deserialization

    #[serde(skip)]
    pub host: Cow<'static, str>,
    #[serde(skip)]
    pub port: u16,
}

impl Status {
    pub fn format<S: AsRef<str>, P: AsRef<str>>(&self, input: S, player_separator: P) -> String {
        input.as_ref().replace("%version", &self.version.name)
            .replace("%description", &self.description.text)
            .replace("%online", &self.players.online.to_string())
            .replace("%max", &self.players.max.to_string())
            .replace("%players", &self.players.to_string(player_separator))
            .replace("%hostname", crate::HOSTNAME.get().unwrap())
            .replace("%host", &self.host)
            .replace("%port", &self.port.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Players {
    pub max: u32,
    pub online: u32,
    pub sample: Option<Vec<Player>>
}

impl Players {
    pub fn to_string<S: AsRef<str>>(&self, separator: S) -> String {
        if let Some(players) = self.sample.as_ref() {
            players.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(separator.as_ref())
        } else {
            "".to_owned()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Description {
    pub text: String,
}

// #[derive(Debug, Deserialize)]
// pub struct ForgeData{
//     pub channels: Vec<ForgeChannel>,
//     pub mods: Vec<Mod>,
//     #[serde(rename(deserialize = "fmlNetworkVersion"))]
//     pub fml_network_version: u8,
//     #[serde(deserialize_with = "de_forge_data")]
//     pub d: Vec<u8>, // FML3 forge data binary data
// }

// pub fn de_forge_data<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error> // TODO: Implement
//     where D: Deserializer<'de>, <D as serde::Deserializer<'de>>::Error: std::error::Error
// {
//     let data = String::deserialize(deserializer)?;
//     let mut chars = data.chars();
//     let length = chars.next().unwrap() as i32 | ((chars.next().unwrap() as i32) << 15);
//     let mut result = Vec::<u8>::new();
//     let mut buffer = 0_u32;
//     let mut bits_in_buffer = 0;
//     while let Some(c) = chars.next() {
//         while bits_in_buffer >= 8 {
//             result.push(buffer as u8);
//             buffer = buffer >> 8;
//             bits_in_buffer -= 8;
//         }
//         buffer |= (c as u32 & 0x7FFF) << bits_in_buffer;
//         bits_in_buffer += 15;
//     }
//     while result.len() < length as usize {
//         result.push(buffer as u8);
//         buffer = buffer >> 8;
//         bits_in_buffer -= 8;
//     }
//     Ok(result)
// }

// #[derive(Debug, Deserialize)]
// pub struct ForgeChannel {
//     pub res: String,
//     pub version: String,
//     pub required: bool,
// }

// #[derive(Debug, Deserialize)]
// pub struct Mod {
//     #[serde(rename(deserialize = "modId"))]
//     pub mod_id: String,
//     #[serde(rename(deserialize = "modmarker"))]
//     pub mod_marker: String,
// }

#[derive(Debug)]
pub struct InternalError {
    message: String
}

impl Error for InternalError {}

impl InternalError {
    pub fn new<S: AsRef<str>>(message: S) -> Self {
        Self { message: message.as_ref().to_owned() }
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
