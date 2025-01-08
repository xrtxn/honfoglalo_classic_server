use std::collections::HashMap;
use std::str::FromStr;

use anyhow::bail;
use serde::{Serialize, Serializer};
use tracing::warn;

use super::county::County;
use super::game::SharedTrivGame;

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
	pub fn serialize_to_hex(&self) -> String {
		let mut ac = self.owner;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		format!("{:02x}", ac)
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

	fn deserialize_from_hex(hex: &str) -> Result<Area, anyhow::Error> {
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
}
impl Serialize for Area {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.serialize_to_hex())
	}
}

impl FromStr for Area {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let byte = u8::from_str_radix(s, 16)?;
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

#[derive(PartialEq, Clone, Debug)]
pub struct Areas(HashMap<County, Area>);

impl Areas {
	pub(crate) fn new() -> Self {
		Areas(HashMap::new())
	}

	pub(crate) fn get_areas(&self) -> &HashMap<County, Area> {
		&self.0
	}

	pub(crate) fn insert(&mut self, county: County, area: Area) {
		self.0.insert(county, area);
	}

	pub(crate) fn get_area(&self, available_county: &County) -> Option<&Area> {
		self.0.get(available_county)
	}

	pub fn serialize(&self) -> Result<String, anyhow::Error> {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(38);
		// start from 1 because we don't want the 0 value County
		for i in 1..20 {
			let county = County::try_from(i)?;
			let area = self.get_area(&county);
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

	pub(crate) fn areas_serializer<S>(counties: &Areas, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		s.serialize_str(&counties.serialize().unwrap())
	}
}

impl FromStr for Areas {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let vals = crate::utils::split_string_n(s, 2);
		let mut rest = Areas::new();
		for (i, county_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't want the 0 value County
				County::try_from((i as u8) + 1)?,
				Area::deserialize_from_hex(county_str)?,
			);
		}
		Ok(rest)
	}
}

impl From<Vec<(County, Area)>> for Areas {
	fn from(counties: Vec<(County, Area)>) -> Self {
		Areas(HashMap::from_iter(counties))
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;
	use serde_test::{assert_ser_tokens, Token};

	use super::*;

	#[test]
	fn full_area_serialize() {
		let res = Areas::serialize(&Areas::from(vec![(
			County::SzabolcsSzatmarBereg,
			// Area::new(PlayerNames::Player3 as u8, false, AreaValue::_1000),
			Area {
				owner: 3,
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
		let res = Areas::from_str("13434343434342424242434141421112414243").unwrap();

		assert_eq!(
			*res.get_area(&County::Pest).unwrap(),
			// Area::new(3 as u8, false, AreaValue::_1000)
			Area {
				owner: 3,
				is_fortress: false,
				value: AreaValue::_1000,
			}
		);

		assert_eq!(
			*res.get_area(&County::SzabolcsSzatmarBereg).unwrap(),
			// Area::new(2, false, AreaValue::_1000)
			Area {
				owner: 2,
				is_fortress: false,
				value: AreaValue::_1000,
			}
		);

		assert_eq!(
			*res.get_area(&County::Baranya).unwrap(),
			// Area::new(1, false, AreaValue::_200
			Area {
				owner: 1,
				is_fortress: false,
				value: AreaValue::_200,
			}
		)
	}

	#[test]
	fn area_test() {
		let area = Area {
			owner: 1,
			is_fortress: false,
			value: AreaValue::_200,
		};
		assert_ser_tokens(&area, &[Token::String("41")]);
		assert_eq!(Area::from_str("41").unwrap(), area);

		let area = Area {
			owner: 3,
			is_fortress: false,
			value: AreaValue::_1000,
		};
		assert_ser_tokens(&area, &[Token::String("13")]);
		assert_eq!(Area::from_str("13").unwrap(), area);

		let area = Area {
			owner: 0,
			is_fortress: false,
			value: AreaValue::Unoccupied,
		};
		assert_ser_tokens(&area, &[Token::String("00")]);
		assert_eq!(Area::from_str("00").unwrap(), area);
	}
}
