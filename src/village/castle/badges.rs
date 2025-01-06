#![allow(clippy::upper_case_acronyms)]

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::emulator::Emulator;

#[derive(Serialize, Deserialize, Debug)]
pub enum BadgeName {
	CW1(u8),
	CW2(u8),
	XPT(u8),
	XPM(u8),
	RLP(u8),
	TWD(u8),
	USQ(u8),
	EXT(u8),
}

impl fmt::Display for BadgeName {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BadgeDetail {}

#[derive(Serialize, Deserialize, Debug)]
pub struct CastleResponse {
	error: String,
	data: Badges,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Badges {
	#[serde(rename = "allbadges")]
	pub castle_badges: Vec<String>,
	#[serde(rename = "castlebadges")]
	pub new_levels: Vec<String>,
}

pub fn all_castle_badges() -> Vec<String> {
	Vec::new()
}

pub fn all_badges() -> Vec<String> {
	Vec::new()
}

impl Emulator for CastleResponse {
	fn emulate() -> Self {
		CastleResponse {
			error: "0".to_string(),
			data: Badges {
				castle_badges: vec![],
				new_levels: vec![],
			},
		}
	}
}
