use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;

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
