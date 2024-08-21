use serde::{Deserialize, Serialize};

use crate::emulator::HungaryEmulator;

#[derive(Serialize, Deserialize)]
#[serde(rename = "ROOT")]
pub struct GameMenuWaithall {
	#[serde(rename = "L")]
	pub l: L,
	#[serde(rename = "STATE")]
	pub state: State,
	#[serde(rename = "GAMEROOM")]
	pub gameroom: Vec<Gameroom>,
	#[serde(rename = "WAITSTATE")]
	pub waitstate: Waitstate,
}

#[derive(Serialize, Deserialize)]
pub struct L {
	#[serde(rename = "@CID")]
	pub cid: String,
	#[serde(rename = "@MN")]
	pub mn: String,
	#[serde(rename = "@R")]
	pub r: String,
}

#[derive(Serialize, Deserialize)]
pub struct State {
	#[serde(rename = "@SCR")]
	pub screen: String,
}

#[derive(Serialize, Deserialize)]
pub struct Gameroom {
	#[serde(rename = "@ID")]
	pub id: String,
	#[serde(rename = "@TITLE")]
	pub title: String,
	#[serde(rename = "@MAP")]
	pub map: String,
	#[serde(rename = "@TYPE")]
	pub gameroom_type: String,
	#[serde(rename = "@PLAYERS")]
	pub players: String,
	#[serde(rename = "@INGAME")]
	pub ingame: String,
	#[serde(rename = "@REMAINING", skip_serializing_if = "Option::is_none")]
	pub remaining_time: Option<u16>,
	// todo complete some tags like closed and maxboosters
	// triviador.swf/triviador.StartWindowMov
}

#[derive(Serialize, Deserialize)]
pub struct Waitstate {
	#[serde(rename = "@ROOMSEL")]
	pub roomsel: String,
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

impl HungaryEmulator for GameMenuWaithall {
	fn emulate(mn: String) -> Self {
		GameMenuWaithall {
			l: L {
				cid: "1".to_string(),
				mn,
				r: "0".to_string(),
			},
			state: State {
				screen: "WAIT".to_string(),
			},
			gameroom: vec![
				Gameroom {
					id: "1".to_string(),
					title: "JUNIOR".to_string(),
					map: "HU".to_string(),
					gameroom_type: "2".to_string(),
					players: "0".to_string(),
					ingame: "0".to_string(),
					remaining_time: None,
				},
				Gameroom {
					id: "2".to_string(),
					title: "DEFAULT".to_string(),
					map: "HU".to_string(),
					gameroom_type: "2".to_string(),
					players: "0".to_string(),
					ingame: "0".to_string(),
					remaining_time: None,
				},
				Gameroom {
					id: "3".to_string(),
					title: "LONG".to_string(),
					map: "HU".to_string(),
					gameroom_type: "2".to_string(),
					players: "0".to_string(),
					ingame: "0".to_string(),
					remaining_time: None,
				},
				// todo
				// Gameroom {
				//     id: "4".to_string(),
				//     title: "MINI".to_string(),
				//     map: "HU".to_string(),
				//     gameroom_type: "10".to_string(),
				//     players: "0".to_string(),
				//     ingame: "0".to_string(),
				//     remaining_time: Some(6000),
				// },
				// Gameroom {
				//     id: "5".to_string(),
				//     title: "FRIENDLY".to_string(),
				//     map: "HU".to_string(),
				//     gameroom_type: "11".to_string(),
				//     players: "0".to_string(),
				//     ingame: "0".to_string(),
				//     remaining_time: None,
				// },
				Gameroom {
					id: "6".to_string(),
					title: "Bajnokság".to_string(),
					map: "HU".to_string(),
					gameroom_type: "12".to_string(),
					players: "0".to_string(),
					ingame: "0".to_string(),
					remaining_time: Some(6000),
				},
			],
			waitstate: Waitstate {
				roomsel: "0".to_string(),
			},
		}
	}
}
