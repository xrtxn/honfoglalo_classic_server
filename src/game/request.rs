use crate::village::start_game;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandRequest {
    #[serde(rename = "CID")]
    pub client_id: String,
    #[serde(rename = "MN")]
    pub mn: String,
    #[serde(rename = "TRY")]
    pub retry_num: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListenRequest {
    #[serde(rename = "CID")]
    pub client_id: String,
    #[serde(rename = "MN")]
    pub mn: String,
    #[serde(rename = "TRY")]
    pub retry_num: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerRequest {
    #[serde(rename = "LOGIN")]
    login: LoginXML,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChangeWHXML {
    #[serde(rename = "@WH")]
    pub waithall: Waithall,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Waithall {
    #[serde(rename = "GAME")]
    Game,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginXML {
    #[serde(rename = "@UID")]
    pub uid: String,
    #[serde(rename = "@NAME")]
    pub name: String,
    #[serde(rename = "@KV")]
    pub kv: String,
    #[serde(rename = "@KD")]
    pub kd: String,
    #[serde(rename = "@LOGINSYSTEM")]
    pub loginsystem: String,
    #[serde(rename = "@CLIENTTYPE")]
    pub clienttype: String,
    #[serde(rename = "@CLIENTVER")]
    pub clientver: String,
    #[serde(rename = "@PAGEMODE")]
    pub pagemode: String,
    #[serde(rename = "@EXTID")]
    pub extid: String,
    #[serde(rename = "@TIME")]
    pub time: String,
    #[serde(rename = "@GUID")]
    pub guid: String,
    #[serde(rename = "@SIGN")]
    pub sign: String,
    #[serde(rename = "@PSID")]
    pub psid: String,
    #[serde(rename = "@CC")]
    pub cc: String,
    #[serde(rename = "@WH")]
    pub wh: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListenXML {
    #[serde(rename = "@READY")]
    pub is_ready: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListenRoot {
    // #[serde(rename = "$value")]
    // pub header_type: Headers,
    #[serde(rename = "$value")]
    pub listen_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandRoot {
    // #[serde(rename = "$value")]
    // pub header_type: Headers,
    #[serde(rename = "$value")]
    pub msg_type: CommandType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "CH")]
pub enum ChannelType {
    #[serde(rename = "C")]
    Command(CommandRequest),
    #[serde(rename = "L")]
    Listen(ListenRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    #[serde(rename = "LISTEN")]
    Listen(ListenXML),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandType {
    #[serde(rename = "LOGIN")]
    Login(LoginXML),
    #[serde(rename = "CHANGEWAITHALL")]
    ChangeWaitHall(ChangeWHXML),
    #[serde(rename = "ENTERROOM")]
    EnterGameLobby(start_game::request::EnterLobbyRequest),
}
