pub mod response {}

pub mod request {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "ROOT")]
	pub struct EnterLobbyRequest {
		#[serde(rename = "@ROOM")]
		pub room: String,
		// todo
		#[serde(rename = "@FRIENDS")]
		pub friends: String,
	}
}
