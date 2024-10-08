use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

use crate::triviador::game_player_data::PlayerNames;
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

	pub fn tower_destroyed(&mut self) {
		self.towers_destroyed += 1;
	}

	pub fn serialize_to_hex(&self) -> String {
		let base_part = self.towers_destroyed << 6;
		crate::utils::to_hex_with_length(&[self.base_id + base_part], 2)
	}

	pub fn deserialize_from_hex(hex: &str) -> Result<Self, anyhow::Error> {
		let value = u8::from_str_radix(hex, 16)?;
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
	every_base: HashMap<PlayerNames, Base>,
}

impl Bases {
	pub async fn get_redis(temp_pool: &RedisPool, game_id: u32) -> Result<Self, anyhow::Error> {
		let res: String = temp_pool
			.hget(format!("games:{}:triviador_state", game_id), "base_info")
			.await?;
		let rest = Self::deserialize_full(&res)?;
		Ok(rest)
	}

	pub async fn set_redis(
		temp_pool: &RedisPool,
		game_id: u32,
		bases: Bases,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[("base_info", Bases::serialize_full(&bases)?)],
			)
			.await?;
		Ok(res)
	}

	pub fn serialize_full(bases: &Bases) -> Result<String, anyhow::Error> {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(6);
		for i in 1..4 {
			match bases.every_base.get(&PlayerNames::try_from(i)?) {
				None => serialized.push_str("00"),
				Some(base) => serialized.push_str(&base.serialize_to_hex()),
			}
		}
		Ok(serialized)
	}

	pub fn deserialize_full(s: &str) -> Result<Self, anyhow::Error> {
		let vals = split_string_n(s, 2);
		let mut rest: HashMap<PlayerNames, Base> = HashMap::with_capacity(3);
		for (i, base_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't have Player0
				PlayerNames::try_from(i as u8 + 1)?,
				Base::deserialize_from_hex(base_str)?,
			);
		}
		Ok(Self { every_base: rest })
	}

	pub fn all_available() -> Self {
		Self {
			every_base: HashMap::new(),
		}
	}

	pub async fn add_base(
		temp_pool: &RedisPool,
		game_id: u32,
		player: PlayerNames,
		base: Base,
	) -> Result<(), anyhow::Error> {
		let mut bases = Bases::get_redis(temp_pool, game_id).await?;

		bases.every_base.insert(player, base);

		Self::set_redis(temp_pool, game_id, bases).await?;
		Ok(())
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
		base.tower_destroyed();
		base.tower_destroyed();
		assert_eq!(base.serialize_to_hex(), "82");
		assert_eq!(Base::deserialize_from_hex("82").unwrap(), base);

		let base = Base::new(8);

		assert_eq!(base.serialize_to_hex(), "08");
		assert_eq!(Base::deserialize_from_hex("08").unwrap(), base);

		let s = "8C080B";
		let res = Bases::deserialize_full(s).unwrap();
		assert_eq!(
			Bases {
				every_base: HashMap::from([
					(
						PlayerNames::Player1,
						Base {
							base_id: 12,
							towers_destroyed: 2
						}
					),
					(
						PlayerNames::Player2,
						Base {
							base_id: 8,
							towers_destroyed: 0
						}
					),
					(
						PlayerNames::Player3,
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
