use std::fmt;
use serde::{Deserialize, Serialize};

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
