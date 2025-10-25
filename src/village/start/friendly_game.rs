use std::str::FromStr;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use serde_aux::prelude::deserialize_number_from_string;
use serde_with::skip_serializing_none;
use tracing::error;

use crate::emulator::Emulator;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExitCurrentRoom {}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) enum OpponentType {
	// i16 num
	Player(i32),
	// 0
	Anyone,
	// -1
	Robot,
	// -2
	Code,
}

impl OpponentType {
	pub(crate) fn get_id(&self) -> i32 {
		match self {
			OpponentType::Player(id) => *id,
			OpponentType::Anyone => 0,
			OpponentType::Robot => -1,
			OpponentType::Code => -2,
		}
	}
}

impl FromStr for OpponentType {
	type Err = std::num::ParseIntError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"0" => Ok(OpponentType::Anyone),
			"-1" => Ok(OpponentType::Robot),
			"-2" => Ok(OpponentType::Code),
			_ => Ok(OpponentType::Player(i32::from_str(s)?)),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFriendlyRoom {
	#[serde(rename = "@OPP1", deserialize_with = "deserialize_number_from_string")]
	pub opp1: OpponentType,
	#[serde(rename = "@OPP2", deserialize_with = "deserialize_number_from_string")]
	pub opp2: OpponentType,
	#[serde(rename = "@NAME1")]
	pub name1: Option<String>,
	#[serde(rename = "@NAME2")]
	pub name2: Option<String>,
	#[serde(rename = "@RULES")]
	pub rules: i8,
	#[serde(rename = "@QCATS")]
	pub question_categories: String,
	#[serde(rename = "@CHATMSG")]
	pub chatmsg: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendlyRoom {
	#[serde(rename = "@ID")]
	pub room_id: u32,
	#[serde(rename = "@PN1")]
	pub player_1: String,
	#[serde(rename = "@NAME1")]
	pub name1: Option<String>,
	#[serde(rename = "@P2")]
	pub player_2: String,
	#[serde(rename = "@P3")]
	pub player_3: String,
}

impl Emulator for FriendlyRoom {
	fn emulate() -> Self {
		FriendlyRoom {
			room_id: 3,
			player_1: "4,0".to_string(),
			name1: Some("Hello".to_string()),
			player_2: "0,0".to_string(),
			player_3: "-1,0".to_string(),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ROOT")]
pub struct FriendlyListRooms {
	#[serde(rename = "ROOM")]
	pub rooms: Vec<FriendlyRoom>,
}

impl Emulator for FriendlyListRooms {
	fn emulate() -> Self {
		FriendlyListRooms {
			rooms: vec![FriendlyRoom::emulate()],
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqFriendlyRoom {
	// max 9999
	#[serde(rename = "@CODE")]
	pub code: Option<u16>,
	/// When joining from the friendly room list
	#[serde(rename = "@ROOM")]
	pub room: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartFriendlyRoom {}

#[skip_serializing_none]
#[derive(Deserialize, Clone, Debug)]
#[serde(rename = "ACTIVESEPROOM")]
pub struct ActiveSepRoom {
	#[serde(rename = "@CODE")]
	pub code: Option<u16>,
	#[serde(rename = "@P1")]
	pub player1: OpponentType,
	pub player1_ready: bool,
	#[serde(rename = "@PN1")]
	pub player1_name: String,
	#[serde(rename = "@P2")]
	pub player2: Option<OpponentType>,
	pub player2_ready: bool,
	#[serde(rename = "@PN2")]
	pub player2_name: Option<String>,
	#[serde(rename = "@P3")]
	pub player3: Option<OpponentType>,
	pub player3_ready: bool,
	#[serde(rename = "@PN3")]
	pub player3_name: Option<String>,
	#[serde(rename = "@STARTDELAY")]
	pub can_start: bool,
}

impl ActiveSepRoom {
	pub(crate) fn new(player1_id: OpponentType, player1_name: &str) -> Self {
		ActiveSepRoom {
			code: None,
			player1: player1_id,
			player1_ready: false,
			player1_name: player1_name.to_string(),
			player2: None,
			player2_ready: false,
			player2_name: None,
			player3: None,
			player3_ready: false,
			player3_name: None,
			can_start: false,
		}
	}

	pub(crate) fn add_opponent(
		&mut self,
		opponent_type: OpponentType,
		name: Option<String>,
	) -> anyhow::Result<()> {
		let is_ready = matches!(opponent_type, OpponentType::Robot);
		let can_replace_code = matches!(opponent_type, OpponentType::Player(_));
		if self.can_add_opponent_to_slot(&self.player2, can_replace_code) {
			self.player2 = Some(opponent_type);
			self.player2_ready = is_ready;
			self.player2_name = name;
			Ok(())
		} else if self.can_add_opponent_to_slot(&self.player3, can_replace_code) {
			self.player3 = Some(opponent_type);
			self.player3_ready = is_ready;
			self.player3_name = name;
			Ok(())
		} else {
			error!("There are three players already in this room! {:?}", self);
			Err(anyhow::anyhow!(
				"There are three players already in this room!"
			))
		}
	}

	fn can_add_opponent_to_slot(
		&self,
		slot: &Option<OpponentType>,
		can_replace_code: bool,
	) -> bool {
		slot.is_none()
			|| (can_replace_code && slot.as_ref().map_or(false, |p| *p == OpponentType::Code))
	}

	pub(crate) fn check_playable(&mut self) {
		if (matches!(self.player1, OpponentType::Robot)
			|| matches!(self.player1, OpponentType::Player(_)))
			&& (matches!(self.player2, Some(OpponentType::Robot))
				|| matches!(self.player2, Some(OpponentType::Player(_))))
			&& (matches!(self.player3, Some(OpponentType::Robot))
				|| matches!(self.player3, Some(OpponentType::Player(_))))
		{
			self.allow_game();
		}
	}

	fn allow_game(&mut self) {
		self.can_start = true
	}
}

impl Serialize for ActiveSepRoom {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct("ACTIVESEPROOM", 9)?;

		if let Some(code) = &self.code {
			state.serialize_field("@CODE", code)?;
		}

		state.serialize_field(
			"@P1",
			&format!("{},{}", self.player1.get_id(), self.player1_ready as u8),
		)?;
		state.serialize_field("@PN1", &self.player1_name)?;

		if let Some(player2_id) = &self.player2 {
			state.serialize_field(
				"@P2",
				&format!("{},{}", player2_id.get_id(), self.player2_ready as u8),
			)?;
		}
		if let Some(name) = &self.player2_name {
			state.serialize_field("@PN2", name)?;
		}
		if let Some(player3_id) = &self.player3 {
			state.serialize_field(
				"@P3",
				&format!("{},{}", player3_id.get_id(), self.player3_ready as u8),
			)?;
		}
		if let Some(name) = &self.player3_name {
			state.serialize_field("@PN3", name)?;
		}

		if self.can_start {
			state.serialize_field("@STARTDELAY", &())?;
		}
		state.end()
	}
}

#[test]
fn friendly_test() {
	let room = ActiveSepRoom {
		code: None,
		player1: OpponentType::Player(1),
		player1_ready: false,
		player1_name: "xrtxn".to_string(),
		player2: Some(OpponentType::Robot),
		player2_ready: false,
		player2_name: None,
		player3: Some(OpponentType::Robot),
		player3_ready: false,
		player3_name: None,
		can_start: false,
	};

	let serialized = quick_xml::se::to_string(&room).unwrap();

	let expected = r#"<ACTIVESEPROOM P1="1,0" PN1="xrtxn" P2="-1,0" P3="-1,0"/>"#;
	assert_eq!(serialized, expected);
}
