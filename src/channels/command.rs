pub mod request {
	use serde::{Deserialize, Serialize};

	use crate::login_screen::LoginXML;
	use crate::menu::friend_list::external_data::ExtDataRequest;
	use crate::village::start::friendly_game::{
		AddFriendlyRoom, ExitCurrentRoom, StartFriendlyRoom,
	};
	use crate::village::start_game::request::EnterLobbyRequest;
	use crate::village::waithall::ChangeWHXML;

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
	pub struct CommandRoot {
		// #[serde(rename = "$value")]
		// pub header_type: Headers,
		#[serde(rename = "$value")]
		pub msg_type: CommandType,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub enum CommandType {
		#[serde(rename = "LOGIN")]
		Login(LoginXML),
		#[serde(rename = "CHANGEWAITHALL")]
		ChangeWaitHall(ChangeWHXML),
		#[serde(rename = "ENTERROOM")]
		EnterGameLobby(EnterLobbyRequest),
		#[serde(rename = "GETEXTDATA")]
		GetExternalData(ExtDataRequest),
		#[serde(rename = "EXITROOM")]
		ExitCurrentRoom(ExitCurrentRoom),
		#[serde(rename = "ADDSEPROOM")]
		AddFriendlyRoom(AddFriendlyRoom),
		#[serde(rename = "STARTSEPROOM")]
		StartTriviador(StartFriendlyRoom),
		#[serde(rename = "READY")]
		PlayerReady,
	}
}

pub mod response {
	use serde::{Deserialize, Serialize};
	use serde_with::skip_serializing_none;

	#[skip_serializing_none]
	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "ROOT")]
	pub struct CommandResponse {
		#[serde(rename = "C")]
		header: CommandResponseHeader,
		// todo this should be the type if it even exists
		message: Option<String>,
	}

	impl CommandResponse {
		pub fn new(header: CommandResponseHeader, message: Option<String>) -> CommandResponse {
			CommandResponse { header, message }
		}

		pub fn ok(cid: impl ToString, mn: impl ToString) -> CommandResponse {
			CommandResponse {
				header: CommandResponseHeader {
					client_id: cid.to_string(),
					mn: mn.to_string(),
					result: 0,
				},
				message: None,
			}
		}
	}

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "C")]
	pub struct CommandResponseHeader {
		#[serde(rename = "@CID")]
		pub client_id: String,
		#[serde(rename = "@MN")]
		pub mn: String,
		#[serde(rename = "@R")]
		pub result: u8,
	}
}
