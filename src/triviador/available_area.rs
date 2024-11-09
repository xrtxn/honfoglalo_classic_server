use std::collections::HashSet;
use std::str::FromStr;

use fred::clients::RedisPool;
use fred::prelude::*;
use tracing::error;

use crate::triviador::county::County;

#[derive(Debug, Clone)]
pub struct AvailableAreas {
	pub areas: HashSet<County>,
}

impl AvailableAreas {
	pub async fn set_empty(temp_pool: &RedisPool, game_id: u32) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			// todo delete old!
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
