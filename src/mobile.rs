pub mod request {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(tag = "cmd")]
	pub enum Mobile {
		#[serde(rename = "ping")]
		Ping(PingRequest),
		#[serde(rename = "login")]
		MobileLogin(MobileLoginRequest),
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct PingRequest {}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct MobileLoginRequest {
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
}

pub mod response {
	use serde::{Deserialize, Serialize};
	use serde_with::skip_serializing_none;

	use crate::emulator::Emulator;
	use crate::mobile::request::{LoginError, Warning};

	#[derive(Serialize, Deserialize, Debug)]
	pub struct PingResponse {
		#[serde(rename = "errormsg")]
		pub message: String,
	}

	impl PingResponse {
		pub(crate) fn pong() -> PingResponse {
			PingResponse {
				message: "pong".to_string(),
			}
		}
	}

	#[skip_serializing_none]
	#[derive(Serialize, Deserialize, Debug)]
	pub struct LoginResult {
		// todo fix warning and error is not diplayed properly in swf
		pub warning: Option<Warning>,
		pub error: Option<LoginError>,
		pub userid: String,
		pub username: String,
		pub userlastname: Option<String>,
		// shouldn't be an option
		pub useremail: String,
		pub guid: String,
		pub sign: String,
		pub time: String,
		pub stoc: String,
		pub currency: Option<String>,
		pub extid: Option<String>,
		pub server: Option<ServerConf>,
		pub sysconf: Option<Sysconf>,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct LoginResponse {
		pub data: LoginResult,
	}

	impl LoginResponse {
		pub(crate) fn nopass_login(username: String) -> LoginResponse {
			LoginResponse {
				data: LoginResult {
					warning: None,
					error: None,
					userid: "".to_string(),
					username,
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

	impl Emulator for LoginResponse {
		fn emulate() -> Self {
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

	#[skip_serializing_none]
	#[derive(Serialize, Deserialize, Debug)]
	pub struct Sysconf {
		#[serde(rename = "ALLOWCHARS")]
		allowed_chars: Option<String>,
		#[serde(rename = "DEVMODE")]
		dev_mode: Option<u8>,
	}

	#[skip_serializing_none]
	#[derive(Serialize, Deserialize, Debug)]
	pub struct ServerConf {
		#[serde(rename = "serveraddress")]
		server_url: Option<String>,
		#[serde(rename = "httpport")]
		http_port: Option<String>,
		#[serde(rename = "xsocketaddress")]
		x_socket_address: Option<String>,
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
}
