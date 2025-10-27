use std::collections::HashMap;
use std::str::FromStr;

use anyhow::bail;
use serde::{Serialize, Serializer};
use tracing::error;

use super::county::County;
use super::game::SharedTrivGame;
use super::game_player_data::PlayerName;

#[derive(Serialize, Clone, PartialEq, Debug)]
pub enum AreaValue {
	Unoccupied = 0,
	_1000 = 1,
	_400 = 2,
	_300 = 3,
	_200 = 4,
}

impl AreaValue {
	pub(crate) fn get_points(&self) -> u16 {
		match self {
			AreaValue::Unoccupied => 0,
			AreaValue::_1000 => 1000,
			AreaValue::_400 => 400,
			AreaValue::_300 => 300,
			AreaValue::_200 => 200,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Area {
	pub(crate) owner: PlayerName,
	is_fortress: bool,
	value: AreaValue,
}

impl Area {
	pub(crate) fn is_castle(&self) -> bool {
		self.value == AreaValue::_1000
	}

	pub(crate) fn get_value(&self) -> &AreaValue {
		&self.value
	}

	pub(crate) fn serialize_to_hex(&self) -> String {
		let mut ac = self.owner as u8;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		format!("{:02x}", ac)
	}

	pub(super) async fn modify_area(
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

	/// Conquer an area from a player
	pub(crate) async fn conquer_area(
		&mut self,
		new_owner: PlayerName,
	) -> Result<(), anyhow::Error> {
		self.owner = new_owner;
		self.upgrade_area();
		Ok(())
	}

	/// Conquer an area from a player
	pub(crate) async fn conquer_area_from_base(
		&mut self,
		new_owner: PlayerName,
	) -> Result<(), anyhow::Error> {
		self.owner = new_owner;
		Ok(())
	}

	pub(crate) fn upgrade_area(&mut self) {
		self.value = self.get_upgrade_value();
	}

	pub(crate) fn get_upgrade_value(&self) -> AreaValue {
		match self.value {
			AreaValue::Unoccupied => {
				error!("Trying to upgrade an unoccupied area!");
				AreaValue::_200
			}
			AreaValue::_200 => AreaValue::_300,
			AreaValue::_300 => AreaValue::_400,
			AreaValue::_400 => AreaValue::_400,
			AreaValue::_1000 => AreaValue::_1000,
		}
	}

	pub(crate) async fn base_selected(
		game: SharedTrivGame,
		player: PlayerName,
		county: County,
	) -> Result<(), anyhow::Error> {
		let base = Self {
			owner: player,
			is_fortress: false,
			value: AreaValue::_1000,
		};
		Self::modify_area(game, (county, base)).await?;
		Ok(())
	}

	pub(crate) async fn area_occupied(
		game: SharedTrivGame,
		rel_id: PlayerName,
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
			error!("Trying to occupy None county!")
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
			owner: PlayerName::from(owner),
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
		Area::deserialize_from_hex(s)
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

	pub(crate) fn get_area_mut(&mut self, available_county: &County) -> Option<&mut Area> {
		self.0.get_mut(available_county)
	}

	/// Conquer all areas of the old owner
	/// Returns the points amount of the conquered areas
	pub(crate) async fn conquer_base_areas(
		&mut self,
		old_owner: PlayerName,
		new_owner: PlayerName,
	) -> u16 {
		let mut total_points: u16 = 0;

		for area in self.0.values_mut() {
			if area.owner == old_owner {
				total_points += area.value.get_points();
				area.conquer_area_from_base(new_owner).await.unwrap();
			}
		}

		total_points
	}

	pub fn serialize(&self) -> String {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(38);
		// start from 1 because we don't want the 0 value County
		for i in 1..=19 {
			let county = County::try_from(i).unwrap();
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
		serialized
	}
}

impl From<Areas> for String {
	fn from(value: Areas) -> Self {
		value.serialize()
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

impl Serialize for Areas {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.serialize().as_str())
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;
	use serde_test::{Token, assert_ser_tokens};

	use super::*;

	#[test]
	fn full_area_serialize() {
		let areas = Areas::from(vec![(
			County::SzabolcsSzatmarBereg,
			Area {
				owner: PlayerName::Player3,
				is_fortress: false,
				value: AreaValue::_1000,
			},
		)]);
		let res: String = areas.into();
		assert_eq!(res, "00000000000000000000000000000013000000");
	}

	#[test]
	fn full_area_deserialize() {
		let res = Areas::from_str("13434343434342424242434141421112414243").unwrap();

		assert_eq!(
			*res.get_area(&County::Pest).unwrap(),
			// Area::new(3 as u8, false, AreaValue::_1000)
			Area {
				owner: PlayerName::Player3,
				is_fortress: false,
				value: AreaValue::_1000,
			}
		);

		assert_eq!(
			*res.get_area(&County::SzabolcsSzatmarBereg).unwrap(),
			// Area::new(2, false, AreaValue::_1000)
			Area {
				owner: PlayerName::Player2,
				is_fortress: false,
				value: AreaValue::_1000,
			}
		);

		assert_eq!(
			*res.get_area(&County::Baranya).unwrap(),
			// Area::new(1, false, AreaValue::_200
			Area {
				owner: PlayerName::Player1,
				is_fortress: false,
				value: AreaValue::_200,
			}
		)
	}

	#[test]
	fn area_test() {
		let area = Area {
			owner: PlayerName::Player1,
			is_fortress: false,
			value: AreaValue::_200,
		};
		assert_ser_tokens(&area, &[Token::String("41")]);
		assert_eq!(Area::from_str("41").unwrap(), area);

		let area = Area {
			owner: PlayerName::Player3,
			is_fortress: false,
			value: AreaValue::_1000,
		};
		assert_ser_tokens(&area, &[Token::String("13")]);
		assert_eq!(Area::from_str("13").unwrap(), area);

		let area = Area {
			owner: PlayerName::Nobody,
			is_fortress: false,
			value: AreaValue::Unoccupied,
		};
		assert_ser_tokens(&area, &[Token::String("00")]);
		assert_eq!(Area::from_str("00").unwrap(), area);
	}
}
