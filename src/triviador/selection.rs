use std::collections::HashMap;

use fred::prelude::*;
use serde::{Serialize, Serializer};

use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::utils::{split_string_n, to_hex_with_length};

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
	counties: HashMap<PlayerNames, County>,
}

impl Selection {
	pub(crate) fn new() -> Self {
		Self {
			counties: HashMap::new(),
		}
	}

	/// clears the redis db
	pub(crate) async fn clear(temp_pool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		Self::set_redis(temp_pool, game_id, Selection::new()).await?;
		Ok(())
	}

	pub async fn get_redis(temp_pool: &RedisPool, game_id: u32) -> Result<Self, anyhow::Error> {
		let res: String = temp_pool
			.get(format!("games:{}:triviador_state:selection", game_id))
			.await?;
		let rest = Self::deserialize_full(&res)?;
		Ok(rest)
	}

	pub async fn set_redis(
		temp_pool: &RedisPool,
		game_id: u32,
		selection: Selection,
	) -> Result<u8, anyhow::Error> {
		let _: String = temp_pool
			.set(
				format!("games:{}:triviador_state:selection", game_id),
				Self::serialize_full(&selection)?,
				None,
				None,
				false,
			)
			.await?;
		Ok(1)
	}

	pub fn add_selection(&mut self, player: PlayerNames, county: County) {
		self.counties.insert(player, county);
	}

	pub fn serialize_full(&self) -> Result<String, anyhow::Error> {
		let mut serialized = String::with_capacity(6);
		// start from 1 because we don't want the 0 value County
		for i in 1..4 {
			let selected_county = self.counties.get(&PlayerNames::try_from(i)?);
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

	pub fn deserialize_full(s: &str) -> Result<Self, anyhow::Error> {
		let vals = split_string_n(s, 2);
		let mut rest: HashMap<PlayerNames, County> = HashMap::with_capacity(3);
		for (i, county_str) in vals.iter().enumerate() {
			let value = u8::from_str_radix(county_str, 16)?;

			rest.insert(
				// increase by 1 because we don't have Player0
				PlayerNames::try_from(i as u8 + 1)?,
				County::try_from(value)?,
			);
		}
		Ok(Self { counties: rest })
	}

	pub(crate) fn get_player_county(&self, rel_id: u8) -> Option<&County> {
		PlayerNames::try_from(rel_id)
			.ok()
			.and_then(|player| self.counties.get(&player))
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
	use crate::triviador::game_player_data::PlayerNames;

	#[test]
	fn test_serialize() {
		let mut selection = Selection::new();
		selection.add_selection(PlayerNames::Player1, County::HajduBihar);
		selection.add_selection(PlayerNames::Player2, County::Veszprem);
		selection.add_selection(PlayerNames::Player3, County::Csongrad);
		let serialized = selection.serialize_full().unwrap();
		assert_eq!(serialized, "090E0B");
	}

	#[test]
	fn test_deserialize() {
		let mut selection = Selection::new();
		selection.add_selection(PlayerNames::Player1, County::HajduBihar);
		selection.add_selection(PlayerNames::Player2, County::Veszprem);
		selection.add_selection(PlayerNames::Player3, County::Csongrad);
		let serialized = Selection::deserialize_full("090E0B").unwrap();
		assert_eq!(serialized, selection);
	}
}
