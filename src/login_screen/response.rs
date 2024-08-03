use crate::emulator::Emulator;
use crate::login_screen::request::{LoginError, Warning};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PingResponse {
    #[serde(rename = "errormsg")]
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LoginError>,
    pub userid: String,
    pub username: String,
    pub userlastname: Option<String>,
    //shouldn't be an option
    pub useremail: String,
    pub guid: String,
    pub sign: String,
    pub time: String,
    pub stoc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extid: Option<String>,
    pub server: Option<ServerConf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysconf: Option<Sysconf>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResponse {
    pub data: LoginResult,
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct CastleResponse {
//     pub error: String,
//     pub data: Badges,
// }

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MobileResponse {
    Ping(PingResponse),
    Login(LoginResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sysconf {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ALLOWCHARS")]
    allowed_chars: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "DEVMODE")]
    dev_mode: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerConf {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "serveraddress")]
    server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "httpport")]
    http_port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "xsocketaddress")]
    x_socket_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "xsocketport")]
    x_socket_port: Option<u16>,
}

// impl Emulator for CastleResponse {
//     fn emulate() -> Self {
//         CastleResponse {
//             error: "0".to_string(),
//             data: Badges {
//                 castle_badges: vec![],
//                 new_levels: vec![],
//             },
//         }
//     }
// }

impl Emulator for LoginResponse {
    fn emulate(_: String) -> Self {
        LoginResponse {
            data: LoginResult {
                warning: None,
                error: None,
                userid: "".to_string(),
                username: "xrtxn".to_string(),
                userlastname: None,
                useremail: "".to_string(),
                guid: "".to_string(),
                sign: "".to_string(),
                time: "".to_string(),
                stoc: "".to_string(),
                currency: None,
                extid: None,
                server: None,
                sysconf: None,
            },
        }
    }
}
