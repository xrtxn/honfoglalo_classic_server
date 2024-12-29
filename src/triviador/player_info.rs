use serde::Serialize;

use crate::triviador::game_player_data::GamePlayerData;

#[derive(Serialize, Debug, Clone)]
pub struct PlayerInfo {
	#[serde(rename = "@P1")]
	pub p1_name: String,
	#[serde(rename = "@P2")]
	pub p2_name: String,
	#[serde(rename = "@P3")]
	pub p3_name: String,
	#[serde(rename = "@PD1")]
	pub pd1: GamePlayerData,
	#[serde(rename = "@PD2")]
	pub pd2: GamePlayerData,
	#[serde(rename = "@PD3")]
	pub pd3: GamePlayerData,
	#[serde(rename = "@YOU")]
	pub you: String,
	#[serde(rename = "@GAMEID")]
	pub game_id: u32,
	#[serde(rename = "@ROOM")]
	pub room: String,
	#[serde(rename = "@RULES")]
	pub rules: String,
}
