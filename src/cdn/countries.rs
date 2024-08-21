use serde::{Deserialize, Serialize};

use crate::cdn::countries::hungary::flags::HunFlags;

pub mod hungary;

#[derive(Serialize, Deserialize, Debug)]
pub struct CountriesResponse {
	pub error: String,
	pub data: Vec<CountriesData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CountriesData {
	pub id: FlagIds,
	pub name: String,
	pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FlagIds {
	Hungarian(HunFlags),
}
