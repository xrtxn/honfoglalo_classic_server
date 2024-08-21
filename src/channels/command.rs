pub mod request {
	use serde::{Deserialize, Serialize};

	use crate::login_screen::LoginXML;
	use crate::players::GetExternalData;
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
		GetExternalData(GetExternalData),
		#[serde(rename = "EXITROOM")]
		ExitCurrentRoom(ExitCurrentRoom),
		#[serde(rename = "ADDSEPROOM")]
		AddFriendlyRoom(AddFriendlyRoom),
		#[serde(rename = "STARTSEPROOM")]
		StartFriendlyRoom(StartFriendlyRoom),
	}
}

pub mod response {
	use serde::{Deserialize, Serialize};

	use crate::emulator::HungaryEmulator;

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

	impl HungaryEmulator for CommandResponse {
		fn emulate(mn: String) -> CommandResponse {
			CommandResponse {
				client_id: "1".to_string(),
				mn,
				result: 0,
			}
		}
	}
}
