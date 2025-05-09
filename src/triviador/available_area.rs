use std::collections::HashSet;

use serde::{Serialize, Serializer};

use super::areas::Areas;
use super::game_player_data::PlayerName;
use super::selection::Selection;
use crate::triviador::county::County;

#[derive(PartialEq, Clone, Debug)]
pub struct AvailableAreas(HashSet<County>);

impl AvailableAreas {
	pub(crate) fn new() -> Self {
		AvailableAreas(HashSet::new())
	}

	pub(crate) fn counties(&self) -> &HashSet<County> {
		&self.0
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Separates the areas into two sets, one for the player and one for the other players
	fn separate_areas(areas: &Areas, player_id: PlayerName) -> (AvailableAreas, AvailableAreas) {
		let mut player_areas = AvailableAreas::new();
		let mut other_areas = AvailableAreas::new();

		for (county, area) in areas.get_areas() {
			if area.owner == player_id {
				player_areas.0.insert(*county);
			} else if area.owner != PlayerName::Nobody {
				other_areas.0.insert(*county);
			}
		}
		dbg!(&player_areas, &other_areas);
		(player_areas, other_areas)
	}

	fn get_neighbouring_areas(&self) -> AvailableAreas {
		let mut filtered_areas = AvailableAreas::new();
		let all_areas = AvailableAreas::all_counties();

		for county in all_areas.0.iter() {
			for player_county in self.0.iter() {
				if player_county.is_neighbour(*county) {
					filtered_areas.0.insert(*county);
				}
			}
		}
		filtered_areas
	}

	fn filter_occupied_areas(&mut self, areas: &Areas) {
		self.0.retain(|available_county| {
			if let Some(area) = areas.get_area(available_county) {
				area.owner == PlayerName::Nobody
			} else {
				true
			}
		});
	}

	fn filter_selected_areas(&mut self, selection: &Selection) {
		for player in PlayerName::all() {
			if let Some(county) = selection.get_player_county(&player) {
				if self.0.contains(county) {
					self.0.remove(county);
				}
			}
		}
	}

	fn filter_player_areas(&mut self, areas: &Areas, rel_id: PlayerName) {
		self.0.retain(|county| {
			if let Some(area) = areas.get_area(county) {
				area.owner != rel_id
			} else {
				false
			}
		});
	}

	pub(crate) fn get_base_areas(areas: &Areas, rel_id: PlayerName) -> AvailableAreas {
		let (_, other_areas) = Self::separate_areas(areas, rel_id);
		let excluded = Self::get_neighbouring_areas(&other_areas);
		let mut player_areas = Self::all_counties();
		player_areas.0.retain(|p| !excluded.0.contains(p));
		player_areas.filter_occupied_areas(areas);
		player_areas
	}

	pub(crate) fn get_conquerable_areas(
		areas: &Areas,
		selection: &Selection,
		rel_player_id: PlayerName,
	) -> AvailableAreas {
		let (mut player_areas, _) = Self::separate_areas(areas, rel_player_id);
		player_areas = Self::get_neighbouring_areas(&player_areas);
		player_areas.filter_occupied_areas(areas);
		player_areas.filter_selected_areas(selection);

		// if there are no filtered available areas, but there are still free areas return all
		// unoccupied areas
		// todo check if there are still free areas
		if player_areas.counties().is_empty() {
			player_areas = AvailableAreas::all_counties();
			player_areas.filter_occupied_areas(areas);
			player_areas.filter_selected_areas(selection);
		}

		player_areas
	}

	pub(crate) fn get_attackable_areas(areas: &Areas, rel_id: PlayerName) -> AvailableAreas {
		let (player_areas, _) = Self::separate_areas(areas, rel_id);
		let mut neighbouring = Self::get_neighbouring_areas(&player_areas);
		// remove player areas
		neighbouring.filter_player_areas(areas, rel_id);
		neighbouring
	}

	/// this does not fail if the removable county is not there
	pub(crate) fn pop_county(&mut self, county: &County) -> bool {
		self.0.remove(county)
	}

	pub(crate) fn push_county(&mut self, county: County) -> bool {
		self.0.insert(county)
	}

	pub(crate) fn all_counties() -> AvailableAreas {
		AvailableAreas(HashSet::from([
			County::Pest,
			County::Nograd,
			County::Heves,
			County::JaszNagykunSzolnok,
			County::BacsKiskun,
			County::Fejer,
			County::KomaromEsztergom,
			County::Borsod,
			County::HajduBihar,
			County::Bekes,
			County::Csongrad,
			County::Tolna,
			County::Somogy,
			County::Veszprem,
			County::GyorMosonSopron,
			County::SzabolcsSzatmarBereg,
			County::Baranya,
			County::Zala,
			County::Vas,
		]))
	}

	pub fn encode_available_areas(areas: Vec<i32>) -> String {
		let mut available: i32 = 0;

		for &area in &areas {
			if (1..=30).contains(&area) {
				available |= 1 << (area - 1);
			}
		}

		// Convert the integer to a byte array (in big-endian format)
		let available_bytes = available.to_be_bytes();

		crate::utils::to_hex_with_length(&available_bytes, 6)
	}
}

impl From<Vec<County>> for AvailableAreas {
	fn from(counties: Vec<County>) -> Self {
		AvailableAreas(counties.into_iter().collect())
	}
}

impl Serialize for AvailableAreas {
	fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		if self.counties().is_empty() {
			return s.serialize_str("000000");
		};
		// there might be more efficient methods than copying but this works for now
		let res = self
			.counties()
			.iter()
			.map(|&county| county as i32)
			.collect();
		s.serialize_str(&Self::encode_available_areas(res))
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use pretty_assertions::assert_eq;

	use super::*;
	use crate::triviador::game_player_data::PlayerName;

	pub fn decode_available_areas(available: i32) -> Vec<i32> {
		let mut res = Vec::new();
		for i in 1..=30 {
			if (available & (1 << (i - 1))) != 0 {
				res.push(i);
			}
		}
		res
	}

	#[test]
	fn get_limited_available() {
		// Szabolcs-Szatmár-Bereg
		let areas = Areas::from_str("00000000000000000000000000000011000000").unwrap();

		let mut selection = Selection::new();

		let available =
			AvailableAreas::get_conquerable_areas(&areas, &selection, PlayerName::Player1);

		assert_eq!(available.0.len(), 2);
		assert!(available.0.contains(&County::Borsod));
		assert!(available.0.contains(&County::HajduBihar));

		let p2_available = AvailableAreas::get_base_areas(&areas, PlayerName::Player2);

		assert_eq!(p2_available.0.len(), 16);
		assert!(!p2_available.0.contains(&County::SzabolcsSzatmarBereg));
		assert!(!p2_available.0.contains(&County::Borsod));
		assert!(!p2_available.0.contains(&County::HajduBihar));

		let areas = Areas::from_str("00001100000000000000000000000000000000").unwrap();
		selection.add_selection(PlayerName::Player2, County::SzabolcsSzatmarBereg);
		let p2_available =
			AvailableAreas::get_conquerable_areas(&areas, &selection, PlayerName::Player2);
		assert_eq!(p2_available.0.len(), 17);
		assert!(!p2_available.0.contains(&County::Heves));
		// because it is selected
		assert!(!p2_available.0.contains(&County::SzabolcsSzatmarBereg));
	}

	#[test]
	fn separate_areas() {
		let areas = Areas::from_str("11000000000000120000000000130000000000").unwrap();

		let sep = AvailableAreas::separate_areas(&areas, PlayerName::Player1);

		assert_eq!(sep.0, AvailableAreas::from(vec![County::Pest]));
		assert_eq!(
			sep.1,
			AvailableAreas::from(vec![County::Borsod, County::Veszprem,])
		);
	}

	#[test]
	fn get_neighbouring_areas() {
		let player_areas = AvailableAreas::from(vec![County::Pest]);

		let filtered = AvailableAreas::get_neighbouring_areas(&player_areas);

		assert_eq!(
			filtered,
			AvailableAreas::from(vec![
				County::Nograd,
				County::Heves,
				County::JaszNagykunSzolnok,
				County::BacsKiskun,
				County::Fejer,
				County::KomaromEsztergom,
			])
		);

		assert!(!filtered.0.contains(&County::Pest));

		let player_areas = AvailableAreas::from(vec![County::Pest, County::SzabolcsSzatmarBereg]);

		let filtered = AvailableAreas::get_neighbouring_areas(&player_areas);

		assert_eq!(
			filtered,
			AvailableAreas::from(vec![
				County::Nograd,
				County::Heves,
				County::JaszNagykunSzolnok,
				County::BacsKiskun,
				County::Fejer,
				County::KomaromEsztergom,
				County::Borsod,
				County::HajduBihar,
			])
		);

		assert!(!filtered.0.contains(&County::Pest));
		assert!(!filtered.0.contains(&County::SzabolcsSzatmarBereg));
	}

	#[test]
	fn attackable_areas() {
		let areas = Areas::from_str("11414241414142124342134342424243414343").unwrap();
		// let player_areas = AvailableAreas::from(vec![County::Pest]);

		let filtered = AvailableAreas::get_attackable_areas(&areas, PlayerName::Player1);

		assert!(filtered.counties().contains(&County::Heves));
		assert!(filtered.counties().contains(&County::Borsod));
		assert!(filtered.counties().contains(&County::HajduBihar));
		assert!(filtered.counties().contains(&County::Bekes));
		assert!(filtered.counties().contains(&County::Csongrad));
		assert!(filtered.counties().contains(&County::Tolna));
		assert!(filtered.counties().contains(&County::Somogy));
		assert!(filtered.counties().contains(&County::Veszprem));
		assert!(filtered.counties().contains(&County::KomaromEsztergom));
		assert!(!filtered.counties().contains(&County::Pest));
		assert!(!filtered.counties().contains(&County::Fejer));
		assert!(!filtered.counties().contains(&County::BacsKiskun));
		assert!(!filtered.counties().contains(&County::Baranya));
		assert!(!filtered.counties().contains(&County::JaszNagykunSzolnok));
		assert!(!filtered.counties().contains(&County::Nograd));
	}
	#[test]
	fn deserializer() {
		let areas = Areas::from_str("11000000000000120000000000130000000000").unwrap();

		assert_eq!(
			areas.get_area(&County::Pest).unwrap().owner,
			PlayerName::Player1
		);
		assert_eq!(
			areas.get_area(&County::Borsod).unwrap().owner,
			PlayerName::Player2
		);
		assert_eq!(
			areas.get_area(&County::Veszprem).unwrap().owner,
			PlayerName::Player3
		);
	}

	#[test]
	fn county_serialize() {
		let decoded = decode_available_areas(i32::from_str_radix("07FFFF", 16).unwrap());
		assert_eq!(
			decoded,
			vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
		);
		assert_eq!(
			AvailableAreas::encode_available_areas(vec![
				1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18, 19
			],),
			"077FFF"
		)
	}
}
