use std::collections::HashMap;

use fred::prelude::*;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
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
	pub question_categories: String,
	#[serde(rename = "@CHATMSG")]
	pub chatmsg: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartFriendlyRoom {}

#[skip_serializing_none]
#[derive(Deserialize, Debug)]
#[serde(rename = "ACTIVESEPROOM")]
pub struct ActiveSepRoom {
	#[serde(rename = "@CODE")]
	pub code: Option<u16>,
	#[serde(rename = "@P1")]
	pub player1_id: i32,
	pub player1_ready: bool,
	#[serde(rename = "@PN1")]
	pub player1_name: String,
	#[serde(rename = "@P2")]
	pub player2_id: i32,
	pub player2_ready: bool,
	#[serde(rename = "@PN2")]
	pub player2_name: Option<String>,
	#[serde(rename = "@P3")]
	pub player3_id: i32,
	pub player3_ready: bool,
	#[serde(rename = "@PN3")]
	pub player3_name: Option<String>,
	#[serde(rename = "@STARTDELAY")]
	pub start_delay: Option<u8>,
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
			&format!("{},{}", self.player1_id, self.player1_ready as u8),
		)?;
		state.serialize_field("@PN1", &self.player1_name)?;

		state.serialize_field(
			"@P2",
			&format!("{},{}", self.player2_id, self.player2_ready as u8),
		)?;
		if let Some(name) = &self.player2_name {
			state.serialize_field("@PN2", name)?;
		}

		state.serialize_field(
			"@P3",
			&format!("{},{}", self.player3_id, self.player3_ready as u8),
		)?;
		if let Some(name) = &self.player3_name {
			state.serialize_field("@PN3", name)?;
		}

		if let Some(start_delay) = &self.start_delay {
			state.serialize_field("@STARTDELAY", start_delay)?;
		}
		state.end()
	}
}

impl ActiveSepRoom {
	pub async fn set_next_num(temp_pool: &RedisPool, number: u32) -> Result<(), anyhow::Error> {
		let _: String = temp_pool
			.set("games:active_rooms:num", number, None, None, false)
			.await?;
		Ok(())
	}
	pub async fn get_next_num(temp_pool: &RedisPool) -> Result<u32, anyhow::Error> {
		let res: u32 = temp_pool
			.get("games:active_rooms:num")
			.await
			.unwrap_or_else(|_| 0);
		Ok(res + 1)
	}

	pub async fn set_active(
		temp_pool: &RedisPool,
		room_number: u32,
		room: ActiveSepRoom,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:active_rooms:{}", room_number),
				[
					("code", room.code.map(|x| x.to_string())),
					("player1_id", Some(room.player1_id.to_string())),
					("player1_ready", Some(room.player1_ready.to_string())),
					("player1_name", Some(room.player1_name)),
					("player2_id", Some(room.player2_id.to_string())),
					("player2_ready", Some(room.player2_ready.to_string())),
					("player2_name", room.player2_name),
					("player3_id", Some(room.player3_id.to_string())),
					("player3_ready", Some(room.player3_ready.to_string())),
					("player3_name", room.player3_name),
					("start_delay", room.start_delay.map(|x| x.to_string())),
				],
			)
			.await?;
		// todo fix possible overflow
		Self::set_next_num(temp_pool, room_number + 1).await?;
		Ok(res)
	}

	pub async fn get_active(
		temp_pool: &RedisPool,
		number: u32,
	) -> Result<ActiveSepRoom, anyhow::Error> {
		let res: HashMap<String, String> = temp_pool
			.hgetall(format!("games:active_rooms:{}", number))
			.await?;
		let room = ActiveSepRoom {
			code: res.get("code").map(|x| x.parse().unwrap()),
			player1_id: res.get("player1_id").unwrap().parse()?,
			player1_ready: res.get("player1_ready").unwrap().parse()?,
			player1_name: res.get("player1_name").unwrap().to_string(),
			player2_id: res.get("player2_id").unwrap().parse()?,
			player2_ready: res.get("player2_ready").unwrap().parse()?,
			player2_name: res.get("player2_name").map(|x| x.to_string()),
			player3_id: res.get("player3_id").unwrap().parse()?,
			player3_ready: res.get("player3_ready").unwrap().parse()?,
			player3_name: res.get("player3_name").map(|x| x.to_string()),
			start_delay: res.get("start_delay").map(|x| x.parse().unwrap()),
		};
		Ok(room)
	}

	pub(crate) async fn new_bots_room(
		temp_pool: &RedisPool,
		room_number: u32,
		player1_id: i32,
		player1_name: &str,
	) -> Result<u8, anyhow::Error> {
		let res = Self::set_active(
			temp_pool,
			room_number,
			ActiveSepRoom {
				code: None,
				player1_id,
				player1_ready: true,
				player1_name: player1_name.to_string(),
				player2_id: -1,
				player2_ready: true,
				player2_name: None,
				player3_id: -1,
				player3_ready: true,
				player3_name: None,
				start_delay: Some(1),
			},
		)
		.await?;
		Ok(res)
	}
}

#[test]
fn friendly_test() {
	let room = ActiveSepRoom {
		code: None,
		player1_id: 1,
		player1_ready: false,
		player1_name: "xrtxn".to_string(),
		player2_id: -1,
		player2_ready: false,
		player2_name: None,
		player3_id: -1,
		player3_ready: false,
		player3_name: None,
		start_delay: None,
	};

	let serialized = quick_xml::se::to_string(&room).unwrap();

	let expected = r#"<ACTIVESEPROOM P1="1,0" PN1="xrtxn" P2="-1,0" P3="-1,0"/>"#;
	assert_eq!(serialized, expected);
}
