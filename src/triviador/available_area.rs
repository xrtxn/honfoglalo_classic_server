use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use fred::clients::RedisPool;
use fred::prelude::*;
use tracing::error;

use crate::triviador::areas::Area;
use crate::triviador::county::County;

#[derive(Debug, Clone)]
pub struct AvailableAreas {
	pub areas: HashSet<County>,
}

impl AvailableAreas {
	pub async fn set_empty(temp_pool: &RedisPool, game_id: u32) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			// todo delete old
			.lpush(
				format!("games:{}:triviador_state:available_areas", game_id),
				[""],
			)
			.await?;
		Ok(res)
	}

	pub async fn set_available(
		temp_pool: &RedisPool,
		game_id: u32,
		areas: AvailableAreas,
	) -> Result<u8, anyhow::Error> {
		let vec: Vec<String> = if areas.areas.is_empty() {
			vec!["".to_string()]
		} else {
			areas
				.areas
				.iter()
				.map(|county| county.to_string())
				.collect::<Vec<String>>()
		};
		// this may be dangerous
		temp_pool
			.del::<u8, _>(format!("games:{}:triviador_state:available_areas", game_id))
			.await?;

		let res = temp_pool
			.rpush::<u8, _, _>(
				format!("games:{}:triviador_state:available_areas", game_id),
				vec,
			)
			.await?;
		Ok(res)
	}
	pub(crate) async fn get_available(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Option<AvailableAreas> {
		let test_areas: Vec<String> = temp_pool
			.lrange(
				format!("games:{}:triviador_state:available_areas", game_id),
				0,
				-1,
			)
			.await
			.unwrap_or_default();

		let available: HashSet<County> = test_areas
			.into_iter()
			.filter_map(|area| County::from_str(&area).ok())
			.collect();

		Some(AvailableAreas { areas: available })
	}

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

	fn get_neighbouring_areas(player_areas: HashSet<County>) -> HashSet<County> {
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

	pub(crate) async fn get_limited_available(
		temp_pool: &RedisPool,
		game_id: u32,
		rel_player_id: u8,
	) -> Option<AvailableAreas> {
		let areas = Area::get_redis(temp_pool, game_id).await.unwrap();
		let player_areas = Self::separate_areas(&areas, rel_player_id).0;
		let filtered = Self::get_neighbouring_areas(player_areas);
		let filtered = Self::filter_occupied_areas(filtered, &areas);

		Some(AvailableAreas { areas: filtered })
	}

	/// this does not fail if the removable county is not there
	pub(crate) async fn pop_county(
		temp_pool: &RedisPool,
		game_id: u32,
		county: County,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.lrem(
				format!("games:{}:triviador_state:available_areas", game_id),
				1,
				county.to_string(),
			)
			.await?;
		if res == 0 {
			error!("County {} was not in the list", county);
		}
		Ok(res)
	}

	pub(crate) async fn push_county(
		temp_pool: &RedisPool,
		game_id: u32,
		county: County,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.rpush(
				format!("games:{}:triviador_state:available_areas", game_id),
				county.to_string(),
			)
			.await?;
		Ok(res)
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
	use super::*;

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

		let filtered = AvailableAreas::get_neighbouring_areas(player_areas);

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

		let filtered = AvailableAreas::get_neighbouring_areas(player_areas);

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
}
