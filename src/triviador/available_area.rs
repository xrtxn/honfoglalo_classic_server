use std::collections::{HashMap, HashSet};

use tracing::trace;
use tracing_subscriber::filter;

use super::game::{SharedTrivGame, TriviadorGame};
use crate::triviador::areas::Area;
use crate::triviador::county::County;

#[derive(Debug, Clone)]
pub struct AvailableAreas {
	pub areas: HashSet<County>,
}

impl AvailableAreas {
	/// Separates the areas into two sets, one for the player and one for the other players
	fn separate_areas(
		areas: &HashMap<County, Area>,
		player_id: u8,
	) -> (HashSet<County>, HashSet<County>) {
		let mut player_areas = HashSet::new();
		let mut other_areas = HashSet::new();

		for (county, area) in areas {
			if area.owner == player_id {
				player_areas.insert(county.clone());
			} else if area.owner != 0 {
				other_areas.insert(county.clone());
			}
		}
		(player_areas, other_areas)
	}

	fn get_neighbouring_areas(player_areas: &HashSet<County>) -> HashSet<County> {
		let mut filtered_areas = HashSet::new();
		let all_areas = AvailableAreas::all_counties();

		for county in all_areas.areas.iter() {
			for player_county in player_areas.iter() {
				if player_county.is_neighbour(county.clone()) {
					filtered_areas.insert(county.clone());
				}
			}
		}
		filtered_areas
	}

	fn filter_occupied_areas(
		available: HashSet<County>,
		areas: &HashMap<County, Area>,
	) -> HashSet<County> {
		let mut filtered = HashSet::new();
		for available_county in available.iter() {
			if let Some(area) = areas.get(available_county) {
				if area.owner == 0 {
					filtered.insert(available_county.clone());
				}
			}
		}
		filtered
	}

	// FIXME
	pub(crate) fn get_limited_available(
		areas: &HashMap<County, Area>,
		rel_player_id: u8,
	) -> Option<AvailableAreas> {
		let (mut player_areas, other_areas) = Self::separate_areas(&areas, rel_player_id);
		if !player_areas.is_empty() {
			player_areas = Self::get_neighbouring_areas(&player_areas);
		} else {
			let excluded = Self::get_neighbouring_areas(&other_areas);
			player_areas = Self::all_counties().areas;
			player_areas.retain(|p| !excluded.contains(p));
		}

		let filtered = Self::filter_occupied_areas(player_areas, &areas);

		Some(AvailableAreas { areas: filtered })
	}

	/// this does not fail if the removable county is not there
	pub(crate) async fn pop_county(game: SharedTrivGame, county: County) -> bool {
		game.write()
			.await
			.state
			.available_areas
			.as_mut()
			.unwrap()
			.areas
			.remove(&county)
	}

	pub(crate) async fn push_county(game: SharedTrivGame, county: County) -> bool {
		game.write()
			.await
			.state
			.available_areas
			.as_mut()
			.unwrap()
			.areas
			.insert(county)
	}

	pub(crate) fn all_counties() -> AvailableAreas {
		AvailableAreas {
			areas: HashSet::from([
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
			]),
		}
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn get_limited_available() {
		let areas =
			Area::deserialize_full("00000000000000000000000000000011000000".to_string()).unwrap();

		let available = AvailableAreas::get_limited_available(&areas, 1).unwrap();

		assert_eq!(available.areas.len(), 2);
		assert!(available.areas.contains(&County::Borsod));
		assert!(available.areas.contains(&County::HajduBihar));

		let p2_available = AvailableAreas::get_limited_available(&areas, 2).unwrap();

		assert_eq!(p2_available.areas.len(), 16);
		assert!(!p2_available.areas.contains(&County::SzabolcsSzatmarBereg));
		assert!(!p2_available.areas.contains(&County::Borsod));
		assert!(!p2_available.areas.contains(&County::HajduBihar));
		assert!(p2_available.areas.contains(&County::SzabolcsSzatmarBereg));

		let areas =
			Area::deserialize_full("00001100000000000000000000000000000000".to_string()).unwrap();
		let p2_available = AvailableAreas::get_limited_available(&areas, 2).unwrap();
		assert_eq!(p2_available.areas.len(), 14);
		assert!(!p2_available.areas.contains(&County::Nograd));
		assert!(!p2_available.areas.contains(&County::Pest));
		assert!(!p2_available.areas.contains(&County::JaszNagykunSzolnok));
		assert!(!p2_available.areas.contains(&County::Borsod));
		assert!(!p2_available.areas.contains(&County::Heves));
	}

	#[test]
	fn separate_areas() {
		let areas =
			Area::deserialize_full("11000000000000120000000000130000000000".to_string()).unwrap();

		let sep = AvailableAreas::separate_areas(&areas, 1);

		assert_eq!(sep.0, HashSet::from([County::Pest]));
		assert_eq!(sep.1, HashSet::from([County::Borsod, County::Veszprem,]));
	}

	#[test]
	fn get_neighbouring_areas() {
		let player_areas = HashSet::from([County::Pest]);

		let filtered = AvailableAreas::get_neighbouring_areas(&player_areas);

		assert_eq!(
			filtered,
			HashSet::from([
				County::Nograd,
				County::Heves,
				County::JaszNagykunSzolnok,
				County::BacsKiskun,
				County::Fejer,
				County::KomaromEsztergom,
			])
		);

		assert!(!filtered.contains(&County::Pest));

		let player_areas = HashSet::from([County::Pest, County::SzabolcsSzatmarBereg]);

		let filtered = AvailableAreas::get_neighbouring_areas(&player_areas);

		assert_eq!(
			filtered,
			HashSet::from([
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

		assert!(!filtered.contains(&County::Pest));
		assert!(!filtered.contains(&County::SzabolcsSzatmarBereg));
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
