use crate::emulator::Emulator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseHeaders {
    #[serde(rename = "C")]
    Command(CommandResponse),
    #[serde(rename = "L")]
    Listen(ListenResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "C")]
pub struct CommandResponse {
    #[serde(rename = "@CID")]
    pub client_id: String,
    #[serde(rename = "@MN")]
    pub mn: String,
    #[serde(rename = "@R")]
    pub result: u8,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "L")]
pub struct ListenResponse {
    #[serde(rename = "@CID")]
    pub client_id: String,
    #[serde(rename = "@MN")]
    pub mn: String,
    #[serde(rename = "@R")]
    pub result: u8,
}

impl Emulator for CommandResponse {
    fn emulate(mn: String) -> CommandResponse {
        CommandResponse {
            client_id: "1".to_string(),
            mn,
            result: 0,
        }
    }
}
