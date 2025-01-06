use std::collections::{HashMap, HashSet};

use tracing::trace;
use tracing_subscriber::filter;

use super::areas;
use super::game::{SharedTrivGame, TriviadorGame};
use super::selection::Selection;
use crate::triviador::areas::Area;
use crate::triviador::county::County;

#[derive(PartialEq, Clone, Debug)]
pub struct AvailableAreas(HashSet<County>);

impl AvailableAreas {
	pub(crate) fn new() -> Self {
		AvailableAreas(HashSet::new())
	}

	pub(crate) fn get_counties(&self) -> &HashSet<County> {
		&self.0
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Separates the areas into two sets, one for the player and one for the other players
	fn separate_areas(
		areas: &HashMap<County, Area>,
		player_id: u8,
	) -> (AvailableAreas, AvailableAreas) {
		let mut player_areas = AvailableAreas::new();
		let mut other_areas = AvailableAreas::new();

		for (county, area) in areas {
			if area.owner == player_id {
				player_areas.0.insert(*county);
			} else if area.owner != 0 {
				other_areas.0.insert(*county);
			}
		}
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

	// todo &mut self
	fn filter_occupied_areas(&mut self, areas: &HashMap<County, Area>) {
		self.0.retain(|available_county| {
			if let Some(area) = areas.get(available_county) {
				area.owner == 0
			} else {
				false
			}
		});
	}

	fn filter_selected_areas(&mut self, selection: &Selection) {
		for player in 1..=3 {
			if let Some(county) = selection.get_player_county(player) {
				if self.0.contains(county) {
					self.0.remove(county);
				}
			}
		}
	}

	pub(crate) fn get_limited_available(
		areas: &HashMap<County, Area>,
		selection: &Selection,
		rel_player_id: u8,
	) -> AvailableAreas {
		let (mut player_areas, other_areas) = Self::separate_areas(areas, rel_player_id);
		if !player_areas.0.is_empty() {
			player_areas = Self::get_neighbouring_areas(&player_areas);
		} else {
			let excluded = Self::get_neighbouring_areas(&other_areas);
			player_areas = Self::all_counties();
			player_areas.0.retain(|p| !excluded.0.contains(p));
		}

		player_areas.filter_occupied_areas(areas);
		player_areas.filter_selected_areas(selection);

		// if there are no filtered available areas, but there are still free areas return all
		// unoccupied areas
		// todo check if there are still free areas
		if player_areas.get_counties().is_empty() {
			player_areas = AvailableAreas::all_counties();
			player_areas.filter_occupied_areas(areas);
			player_areas.filter_selected_areas(selection);
		}

		player_areas
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
}

impl From<Vec<County>> for AvailableAreas {
	fn from(counties: Vec<County>) -> Self {
		AvailableAreas(counties.into_iter().collect())
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;
	use crate::triviador::game_player_data::PlayerNames;

	#[test]
	fn get_limited_available() {
		let areas =
			Area::deserialize_full("00000000000000000000000000000011000000".to_string()).unwrap();
		let mut selection = Selection::new();

		let available = AvailableAreas::get_limited_available(&areas, &selection, 1);

		assert_eq!(available.0.len(), 2);
		assert!(available.0.contains(&County::Borsod));
		assert!(available.0.contains(&County::HajduBihar));

		let p2_available = AvailableAreas::get_limited_available(&areas, &selection, 2);

		assert_eq!(p2_available.0.len(), 16);
		assert!(!p2_available.0.contains(&County::SzabolcsSzatmarBereg));
		assert!(!p2_available.0.contains(&County::Borsod));
		assert!(!p2_available.0.contains(&County::HajduBihar));

		let areas =
			Area::deserialize_full("00001100000000000000000000000000000000".to_string()).unwrap();
		selection.add_selection(PlayerNames::Player2, County::SzabolcsSzatmarBereg);
		let p2_available = AvailableAreas::get_limited_available(&areas, &selection, 2);
		assert_eq!(p2_available.0.len(), 13);
		assert!(!p2_available.0.contains(&County::Nograd));
		assert!(!p2_available.0.contains(&County::Pest));
		assert!(!p2_available.0.contains(&County::JaszNagykunSzolnok));
		assert!(!p2_available.0.contains(&County::Borsod));
		assert!(!p2_available.0.contains(&County::Heves));
		// because it is selected
		assert!(!p2_available.0.contains(&County::SzabolcsSzatmarBereg));
	}

	#[test]
	fn separate_areas() {
		let areas =
			Area::deserialize_full("11000000000000120000000000130000000000".to_string()).unwrap();

		let sep = AvailableAreas::separate_areas(&areas, 1);

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
	fn deserializer() {
		let areas =
			Area::deserialize_full("11000000000000120000000000130000000000".to_string()).unwrap();

		assert_eq!(areas.get(&County::Pest).unwrap().owner, 1);
		assert_eq!(areas.get(&County::Borsod).unwrap().owner, 2);
		assert_eq!(areas.get(&County::Veszprem).unwrap().owner, 3);
	}
}
