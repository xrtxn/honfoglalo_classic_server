use std::collections::HashMap;
use std::str::FromStr;

use serde::{Serialize, Serializer};

use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerName;
use crate::utils::{split_string_n, to_hex_with_length};

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
	counties: HashMap<PlayerName, County>,
}

impl Selection {
	pub(crate) fn new() -> Self {
		Self {
			counties: HashMap::new(),
		}
	}

	pub(crate) fn clear(&mut self) {
		self.counties.clear();
	}

	pub fn add_selection(&mut self, player: PlayerName, county: County) {
		self.counties.insert(player, county);
	}

	pub fn get_selection(&self, player: &PlayerName) -> Option<&County> {
		self.counties.get(player)
	}

	pub fn serialize_full(&self) -> Result<String, anyhow::Error> {
		let mut serialized = String::with_capacity(6);
		// start from 1 because we don't want the 0 value County
		for i in 1..=3 {
			let selected_county = self.counties.get(&PlayerName::from(i));
			match selected_county {
				None => {
					serialized.push_str("00");
				}
				Some(county) => {
					let base_part = *county as u8;
					let bytes = base_part.to_be_bytes();
					serialized.push_str(to_hex_with_length(bytes.as_slice(), 2).as_str());
				}
			}
		}
		Ok(serialized)
	}

	pub(crate) fn get_player_county(&self, player: &PlayerName) -> Option<&County> {
		self.counties.get(player)
	}
}

impl FromStr for Selection {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let vals = split_string_n(s, 2);
		let mut rest: HashMap<PlayerName, County> = HashMap::with_capacity(3);
		for (i, county_str) in vals.iter().enumerate() {
			let value = u8::from_str_radix(county_str, 16)?;

			rest.insert(
				// increase by 1 because we don't have Player0
				PlayerName::from(i as u8 + 1),
				County::try_from(value)?,
			);
		}
		Ok(Self { counties: rest })
	}
}

impl Serialize for Selection {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&Self::serialize_full(self).unwrap())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::triviador::county::County;
	use crate::triviador::game_player_data::PlayerName;

	#[test]
	fn test_serialize() {
		let mut selection = Selection::new();
		selection.add_selection(PlayerName::Player1, County::HajduBihar);
		selection.add_selection(PlayerName::Player2, County::Veszprem);
		selection.add_selection(PlayerName::Player3, County::Csongrad);
		let serialized = selection.serialize_full().unwrap();
		assert_eq!(serialized, "090E0B");
	}

	#[test]
	fn test_deserialize() {
		let mut selection = Selection::new();
		selection.add_selection(PlayerName::Player1, County::HajduBihar);
		selection.add_selection(PlayerName::Player2, County::Veszprem);
		selection.add_selection(PlayerName::Player3, County::Csongrad);
		let serialized = Selection::from_str("090E0B").unwrap();
		assert_eq!(serialized, selection);
	}

	#[test]
	fn test_bases() {
		let mut selection = Selection::new();
		selection.add_selection(PlayerName::Player1, County::HajduBihar);
		selection.add_selection(PlayerName::Player2, County::Veszprem);
		selection.add_selection(PlayerName::Player3, County::Csongrad);
		let serialized = Selection::from_str("090E0B").unwrap();
		assert_eq!(serialized, selection);
	}
}
