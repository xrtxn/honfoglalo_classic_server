use serde::{Deserialize, Serialize};
use crate::village::castle::badges::BadgeName;

#[derive(Serialize, Deserialize, Debug)]
pub struct PingResponse {
	#[serde(rename = "errormsg")]
	pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
//todo wrong
#[serde(rename = "castlebadges")]
pub struct Badges {
	#[serde(rename = "castlebadges")]
	pub castle_badges: Vec<String>,
	#[serde(rename = "allbadges")]
	pub other_badges: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewBadgeLevels {
	pub vec: Vec<BadgeName>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CastleResponse {
	pub error: String,
	pub data: Badges,
	#[serde(rename = "NEWLEVELS")]
	pub new_levels: NewBadgeLevels,
}
