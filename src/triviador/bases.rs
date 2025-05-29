use std::collections::HashMap;
use std::str::FromStr;

use serde::{Serialize, Serializer};

use super::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::utils::split_string_n;

#[derive(PartialEq, Debug, Clone)]
pub struct Base {
	base_id: u8,
	towers_destroyed: u8,
}

impl Base {
	pub fn new(base_id: u8) -> Self {
		Self {
			base_id,
			towers_destroyed: 0,
		}
	}

	#[allow(dead_code)]
	pub fn destroy_tower(&mut self) {
		self.towers_destroyed += 1;
	}

	pub fn tower_count(&self) -> u8 {
		3 - self.towers_destroyed
	}

	pub fn serialize_to_hex(&self) -> String {
		let base_part = self.towers_destroyed << 6;
		crate::utils::to_hex_with_length(&[self.base_id + base_part], 2)
	}
}

impl FromStr for Base {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let value = u8::from_str_radix(s, 16)?;
		let towers_destroyed = value >> 6;
		let base_id = value & 0b0011_1111;
		Ok(Base {
			base_id,
			towers_destroyed,
		})
	}
}

impl Serialize for Base {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.serialize_to_hex())
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bases {
	every_base: HashMap<PlayerName, Base>,
}

impl Bases {
	pub(crate) fn serialize_full(&self) -> Result<String, anyhow::Error> {
		let mut serialized = String::with_capacity(6);
		for i in 1..4_u8 {
			match self.every_base.get(&PlayerName::from(i)) {
				None => serialized.push_str("00"),
				Some(base) => serialized.push_str(&base.serialize_to_hex()),
			}
		}
		Ok(serialized)
	}

	pub(crate) fn all_available() -> Self {
		Self {
			every_base: HashMap::new(),
		}
	}

	pub(crate) async fn add_base(
		game: SharedTrivGame,
		player: PlayerName,
		base: Base,
	) -> Result<(), anyhow::Error> {
		game.write()
			.await
			.state
			.base_info
			.every_base
			.insert(player, base);
		Ok(())
	}

	pub(crate) fn get_base(&self, player: &PlayerName) -> Option<&Base> {
		self.every_base.get(player)
	}

	pub(crate) fn get_base_mut(&mut self, player: &PlayerName) -> Option<&mut Base> {
		self.every_base.get_mut(player)
	}

	#[allow(dead_code)]
	pub(crate) fn get_all_bases(&self) -> &HashMap<PlayerName, Base> {
		&self.every_base
	}
}

impl FromStr for Bases {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let vals = split_string_n(s, 2);
		let mut rest: HashMap<PlayerName, Base> = HashMap::with_capacity(3);
		for (i, base_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't have Player0
				PlayerName::try_from(i as u8 + 1)?,
				Base::from_str(base_str)?,
			);
		}
		Ok(Self { every_base: rest })
	}
}

impl Serialize for Bases {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&Bases::serialize_full(self).unwrap())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn base_test() {
		let mut base = Base::new(2);
		base.destroy_tower();
		base.destroy_tower();
		assert_eq!(base.serialize_to_hex(), "82");
		assert_eq!(Base::from_str("82").unwrap(), base);

		let base = Base::new(8);

		assert_eq!(base.serialize_to_hex(), "08");
		assert_eq!(Base::from_str("08").unwrap(), base);

		let s = "8C080B";
		let res = Bases::from_str(s).unwrap();
		assert_eq!(
			Bases {
				every_base: HashMap::from([
					(
						PlayerName::Player1,
						Base {
							base_id: 12,
							towers_destroyed: 2
						}
					),
					(
						PlayerName::Player2,
						Base {
							base_id: 8,
							towers_destroyed: 0
						}
					),
					(
						PlayerName::Player3,
						Base {
							base_id: 11,
							towers_destroyed: 0
						}
					)
				])
			},
			res
		);
	}
}
