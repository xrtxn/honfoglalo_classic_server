use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::emulator::Emulator;

#[derive(Serialize, Deserialize)]
#[serde(rename = "ROOT")]
pub struct GameMenuWaithall {
	#[serde(rename = "STATE")]
	pub state: State,
	#[serde(rename = "GAMEROOM")]
	pub gameroom: Vec<Gameroom>,
	#[serde(rename = "WAITSTATE")]
	pub waitstate: Waitstate,
}

#[derive(Serialize, Deserialize)]
pub struct State {
	#[serde(rename = "@SCR")]
	pub screen: String,
}

#[skip_serializing_none]
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
	#[serde(rename = "@REMAINING")]
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

impl Emulator for GameMenuWaithall {
	fn emulate() -> Self {
		GameMenuWaithall {
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
