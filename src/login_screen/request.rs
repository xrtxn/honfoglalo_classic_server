use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd")]
pub enum Mobile {
    #[serde(rename = "ping")]
    Ping(PingRequest),
    #[serde(rename = "login")]
    Login(LoginRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    #[serde(rename = "clientver")]
    pub client_version: u32,
    #[serde(rename = "loginname")]
    pub username: String,
    pub password: String,
    pub system: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Warning {
    pub message: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginError {
    pub message: String,
}
