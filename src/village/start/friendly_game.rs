use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use serde_with::skip_serializing_none;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExitCurrentRoom {}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFriendlyRoom {
	#[serde(rename = "@OPP1", deserialize_with = "deserialize_number_from_string")]
	pub opp1: i8,
	#[serde(rename = "@OPP2", deserialize_with = "deserialize_number_from_string")]
	pub opp2: i8,
	#[serde(rename = "@NAME1")]
	pub name1: String,
	#[serde(rename = "@NAME2")]
	pub name2: String,
	#[serde(rename = "@RULES")]
	pub rules: i8,
	#[serde(rename = "@QCATS")]
	pub qcats: String,
	#[serde(rename = "@CHATMSG")]
	pub chatmsg: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartFriendlyRoom {}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ACTIVESEPROOM")]
pub struct ActiveSepRoom {
	#[serde(rename = "@CODE")]
	pub code: String,
	#[serde(rename = "@P1")]
	pub p1: String,
	#[serde(rename = "@PN1")]
	pub pn1: String,
	#[serde(rename = "@P2")]
	pub p2: String,
	#[serde(rename = "@P3")]
	pub p3: String,
	#[serde(rename = "@STARTDELAY")]
	pub start_delay: Option<u8>,
}

impl ActiveSepRoom {
	// todo pass a struct
	pub(crate) fn new_bots_room(p_id: u8, player_name: String) -> ActiveSepRoom {
		ActiveSepRoom {
			code: "1234".to_string(),
			p1: format!("{},0", p_id),
			pn1: player_name,
			p2: "-1,0".to_string(),
			p3: "-1,0".to_string(),
			start_delay: None,
		}
	}

	pub(crate) fn start_friendly_room(&mut self) {
		self.start_delay = Some(1);
	}
}

// pub fn starting_emu() -> ActiveSepRoom {
// 	ActiveSepRoom {
// 		code: "1234".to_string(),
// 		p1: "1,0".to_string(),
// 		pn1: "xrtxn".to_string(),
// 		p2: "-1,0".to_string(),
// 		p3: "-1,0".to_string(),
// 		start_delay: Some("1".to_string()),
// 	}
// }
//
// pub fn ready_emu() -> ActiveSepRoom {
// 	ActiveSepRoom {
// 		code: "1234".to_string(),
// 		p1: "1,0".to_string(),
// 		pn1: "xrtxn".to_string(),
// 		p2: "-1,0".to_string(),
// 		p3: "-1,0".to_string(),
// 		start_delay: None,
// 	}
// }
