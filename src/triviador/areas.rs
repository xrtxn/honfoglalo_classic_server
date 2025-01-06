use std::collections::HashMap;

use anyhow::bail;
use serde::{Serialize, Serializer};
use tracing::warn;

use super::game::{SharedTrivGame, TriviadorGame};
use crate::triviador::county::County;
use crate::utils::split_string_n;

#[derive(Serialize, Clone, PartialEq, Debug)]
pub enum AreaValue {
	Unoccupied = 0,
	_1000 = 1,
	_400 = 2,
	_300 = 3,
	_200 = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Area {
	pub(crate) owner: u8,
	is_fortress: bool,
	value: AreaValue,
}

impl Area {
	pub fn new() -> HashMap<County, Area> {
		let mut areas = HashMap::new();
		for i in 1..=19 {
			if let Ok(county) = County::try_from(i) {
				areas.insert(
					county,
					Area {
						owner: 0,
						is_fortress: false,
						value: AreaValue::Unoccupied,
					},
				);
			}
		}
		areas
	}

	pub fn serialize_to_hex(&self) -> String {
		let mut ac = self.owner;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		format!("{:02x}", ac)
	}

	pub fn deserialize_from_hex(hex: &str) -> Result<Self, anyhow::Error> {
		let byte = u8::from_str_radix(hex, 16)?;
		let owner = byte & 0x0F;
		let value = (byte >> 4) & 0x07;
		let is_fortress = (byte & 0x80) != 0;
		let value = AreaValue::try_from(value)?;

		Ok(Area {
			owner,
			is_fortress,
			value,
		})
	}

	pub fn deserialize_full(s: String) -> Result<HashMap<County, Area>, anyhow::Error> {
		let vals = split_string_n(&s, 2);
		let mut rest: HashMap<County, Area> = HashMap::with_capacity(19);
		for (i, county_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't want the 0 value County
				County::try_from((i as u8) + 1)?,
				Area::deserialize_from_hex(county_str)?,
			);
		}
		Ok(rest)
	}

	pub fn serialize_full(set_counties: &HashMap<County, Area>) -> Result<String, anyhow::Error> {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(38);
		// start from 1 because we don't want the 0 value County
		for i in 1..20 {
			let county = County::try_from(i)?;
			let area = set_counties.get(&county);
			match area {
				None => {
					serialized.push_str("00");
				}
				Some(area) => {
					serialized.push_str(&area.serialize_to_hex());
				}
			}
		}
		Ok(serialized)
	}

	pub async fn modify_area(
		game: SharedTrivGame,
		values: (County, Area),
	) -> Result<Option<Area>, anyhow::Error> {
		game.write()
			.await
			.state
			.areas_info
			.insert(values.0, values.1);
		// todo check this out
		Ok(None)
	}

	pub async fn base_selected(
		game: SharedTrivGame,
		game_player_id: u8,
		county: County,
	) -> Result<(), anyhow::Error> {
		let base = Self {
			owner: game_player_id,
			is_fortress: false,
			value: AreaValue::_1000,
		};
		Self::modify_area(game, (county, base)).await?;
		Ok(())
	}

	pub async fn area_occupied(
		game: SharedTrivGame,
		rel_id: u8,
		county: Option<County>,
	) -> Result<(), anyhow::Error> {
		if let Some(county) = county {
			let base = Self {
				owner: rel_id,
				is_fortress: false,
				value: AreaValue::_200,
			};
			Self::modify_area(game, (county, base)).await?;
		} else {
			warn!("Trying to occupy None county!")
		}

		Ok(())
	}
}
impl Serialize for Area {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.serialize_to_hex())
	}
}

pub(crate) fn areas_full_serializer<S>(
	counties: &HashMap<County, Area>,
	s: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	s.serialize_str(&Area::serialize_full(counties).unwrap())
}

impl TryFrom<u8> for AreaValue {
	type Error = anyhow::Error;

	fn try_from(value: u8) -> Result<Self, anyhow::Error> {
		match value {
			0 => Ok(AreaValue::Unoccupied),
			1 => Ok(AreaValue::_1000),
			2 => Ok(AreaValue::_400),
			3 => Ok(AreaValue::_300),
			4 => Ok(AreaValue::_200),
			_ => bail!("Failed to deserialize u8 to AreaValue"),
		}
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_ser_tokens, Token};

	use super::*;
	use crate::triviador::game_player_data::PlayerNames;

	#[test]
	fn area_test() {
		let area = Area {
			owner: 1,
			is_fortress: false,
			value: AreaValue::_200,
		};

		assert_ser_tokens(&area, &[Token::String("41")]);
		assert_eq!(Area::deserialize_from_hex("41").unwrap(), area);

		let area = Area {
			owner: 3,
			is_fortress: false,
			value: AreaValue::_1000,
		};

		assert_ser_tokens(&area, &[Token::String("13")]);
		let area = Area {
			owner: 0,
			is_fortress: false,
			value: AreaValue::Unoccupied,
		};

		assert_ser_tokens(&area, &[Token::String("00")]);
	}

	#[test]
	fn full_area_serialize() {
		let res = Area::serialize_full(&HashMap::from([(
			County::SzabolcsSzatmarBereg,
			Area {
				owner: PlayerNames::Player3 as u8,
				is_fortress: false,
				value: AreaValue::_1000,
			},
		)]))
		.unwrap();
		assert_eq!(res, "00000000000000000000000000000013000000");
	}

	#[test]
	fn full_area_deserialize() {
		// todo this may be an invalid string
		let res =
			Area::deserialize_full("13434343434342424242434141421112414243".to_string()).unwrap();

		assert_eq!(
			*res.get(&County::Pest).unwrap(),
			Area {
				owner: 3,
				is_fortress: false,
				value: AreaValue::_1000
			}
		);

		assert_eq!(
			*res.get(&County::SzabolcsSzatmarBereg).unwrap(),
			Area {
				owner: 2,
				is_fortress: false,
				value: AreaValue::_1000
			}
		);
		assert_eq!(
			*res.get(&County::Baranya).unwrap(),
			Area {
				owner: 1,
				is_fortress: false,
				value: AreaValue::_200
			}
		)
	}
}
