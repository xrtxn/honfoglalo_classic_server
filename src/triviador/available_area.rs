use std::collections::HashSet;
use std::str::FromStr;

use fred::clients::RedisPool;
use fred::prelude::*;

use crate::triviador::county::County;

#[derive(Debug, Clone)]
pub struct AvailableAreas {
	pub areas: HashSet<County>,
}

impl AvailableAreas {
	pub async fn set_empty(tmppool: &RedisPool, game_id: u32) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			// todo delete old!
			.lpush(
				format!("games:{}:triviador_state:available_areas", game_id),
				[""],
			)
			.await?;
		Ok(res)
	}

	pub async fn set_available(
		tmppool: &RedisPool,
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
		tmppool
			.del::<u8, _>(format!("games:{}:triviador_state:available_areas", game_id))
			.await?;

		let res = tmppool
			.rpush::<u8, _, _>(
				format!("games:{}:triviador_state:available_areas", game_id),
				vec,
			)
			.await?;
		Ok(res)
	}
	pub(crate) async fn get_available(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<Option<AvailableAreas>, anyhow::Error> {
		let test_areas: Vec<String> = tmppool
			.lrange(
				format!("games:{}:triviador_state:available_areas", game_id),
				0,
				-1,
			)
			.await?;
		let available: HashSet<County> = if test_areas.contains(&"".to_string())
			&& test_areas.len() == 1
			&& !test_areas.is_empty()
		{
			HashSet::new()
		} else {
			test_areas
				.iter()
				.map(|area| County::from_str(area).unwrap())
				.collect()
		};
		let available = AvailableAreas { areas: available };
		Ok(Some(available))
	}

	/// this does not fail if the removable county is not there
	pub(crate) async fn pop_county(
		tmppool: &RedisPool,
		game_id: u32,
		county: County,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.lrem(
				format!("games:{}:triviador_state:available_areas", game_id),
				1,
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
