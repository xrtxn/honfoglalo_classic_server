use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use serde_with::{BoolFromInt, serde_as};

use crate::emulator::Emulator;

#[derive(Serialize, Deserialize)]
#[serde(rename = "ROOT")]
pub struct GameMenuWaithall {
	#[serde(rename = "STATE")]
	pub state: State,
	#[serde(rename = "GAMEROOM")]
	pub gameroom: Vec<GameRoom>,
	#[serde(rename = "WAITSTATE")]
	pub waitstate: WaitState,
}

#[derive(Serialize, Deserialize)]
pub struct State {
	#[serde(rename = "@SCR")]
	pub screen: String,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct GameRoom {
	#[serde(rename = "@ID")]
	pub id: u8,
	#[serde(rename = "@TITLE")]
	pub title: String,
	#[serde(rename = "@MAP")]
	pub map: String,
	#[serde(rename = "@TYPE")]
	pub game_room_type: u8,
	#[serde(rename = "@PLAYERS")]
	pub players: u16,
	#[serde(rename = "@INGAME")]
	pub ingame: u16,
	/// Seconds remaining
	#[serde(rename = "@REMAINING")]
	pub remaining_time: Option<u16>,
	#[serde_as(as = "Option<BoolFromInt>")]
	#[serde(rename = "@JOINED")]
	pub joined: Option<bool>,
	#[serde_as(as = "Option<BoolFromInt>")]
	#[serde(rename = "@CLOSED")]
	pub closed: Option<bool>,
	// todo complete some tags like maxboosters
	// triviador.swf/triviador.StartWindowMov
}

#[derive(Serialize, Deserialize)]
pub struct WaitState {
	#[serde(rename = "@ROOMSEL")]
	pub roomsel: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChangeWHXML {
	#[serde(rename = "@WH")]
	pub waithall: Waithall,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Waithall {
	#[serde(rename = "GAME")]
	Game,
	#[serde(rename = "VILLAGE")]
	Village,
}

impl Emulator for GameMenuWaithall {
	fn emulate() -> Self {
		GameMenuWaithall {
			state: State {
				screen: "WAIT".to_string(),
			},
			gameroom: vec![
				GameRoom {
					id: 1,
					title: "JUNIOR".to_string(),
					map: "HU".to_string(),
					game_room_type: 2,
					players: 0,
					ingame: 0,
					remaining_time: Some(100),
					joined: None,
					closed: None,
				},
				GameRoom {
					id: 2,
					title: "DEFAULT".to_string(),
					map: "HU".to_string(),
					game_room_type: 2,
					players: 0,
					ingame: 0,
					remaining_time: None,
					joined: None,
					closed: None,
				},
				GameRoom {
					id: 3,
					title: "LONG".to_string(),
					map: "HU".to_string(),
					game_room_type: 2,
					players: 0,
					ingame: 0,
					remaining_time: None,
					joined: None,
					closed: None,
				},
				GameRoom {
					id: 4,
					title: "MINI".to_string(),
					map: "HU".to_string(),
					game_room_type: 10,
					players: 0,
					ingame: 0,
					remaining_time: Some(400),
					joined: None,
					closed: Some(true),
				},
				GameRoom {
					id: 5,
					title: "Bajnokság".to_string(),
					map: "HU".to_string(),
					game_room_type: 12,
					players: 0,
					ingame: 0,
					remaining_time: Some(500),
					joined: Some(false),
					closed: Some(true),
				},
			],
			waitstate: WaitState { roomsel: 0 },
		}
	}
}
