use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

use crate::triviador::available_area::AvailableAreas;
use crate::utils::to_hex_with_length;
#[derive(Debug, Serialize, Clone)]
pub struct Cmd {
	#[serde(rename = "@CMD")]
	pub command: String,
	#[serde(rename = "@AVAILABLE", serialize_with = "available_serialize")]
	pub available: Option<AvailableAreas>,
	#[serde(rename = "@TO")]
	// seconds for action
	pub timeout: u8,
}

impl Cmd {
	pub async fn set_player_cmd(
		tmppool: &RedisPool,
		player_id: i32,
		cmd: Cmd,
	) -> Result<u8, anyhow::Error> {
		{
			let res: u8 = tmppool
				.hset(
					format!("users:{}:cmd", player_id),
					[
						("command", cmd.command),
						("cmd_timeout", cmd.timeout.to_string()),
					],
				)
				.await?;
			Ok(res)
		}
	}
	/// Gets a player's requested command, if none returns None
	/// Gets the available areas from the triviador game state
	pub(crate) async fn get_player_cmd(
		tmppool: &RedisPool,
		player_id: i32,
		game_id: u32,
	) -> Result<Option<Cmd>, anyhow::Error> {
		let res: HashMap<String, String> =
			tmppool.hgetall(format!("users:{}:cmd", player_id)).await?;

		// return if none
		if res.is_empty() {
			return Ok(None);
		}

		let available = AvailableAreas::get_available(tmppool, game_id).await?;

		Ok(Some(Cmd {
			command: res.get("command").unwrap().to_string(),
			available,
			timeout: res.get("cmd_timeout").unwrap().parse()?,
		}))
	}

	pub(crate) async fn clear_cmd(
		tmppool: &RedisPool,
		player_id: i32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool.del(format!("users:{}:cmd", player_id)).await?;
		Ok(res)
	}
}

#[allow(dead_code)]
#[derive(Serialize, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum County {
	NoResponse = 0,            // if nothing is selected
	Pest = 1,                  // Pest
	Nograd = 2,                // Nógrád
	Heves = 3,                 // Heves
	JaszNagykunSzolnok = 4,    // Jász-Nagykun-Szolnok
	BacsKiskun = 5,            // Bács-Kiskun
	Fejer = 6,                 // Fejér
	KomaromEsztergom = 7,      // Komárom-Esztergom
	Borsod = 8,                // Borsod
	HajduBihar = 9,            // Hajdú-Bihar
	Bekes = 10,                // Békés
	Csongrad = 11,             // Csongrád
	Tolna = 12,                // Tolna
	Somogy = 13,               // Somogy
	Veszprem = 14,             // Veszprém
	GyorMosonSopron = 15,      // Győr-Moson-Sopron
	SzabolcsSzatmarBereg = 16, // Szabolcs-Szatmár-Bereg
	Baranya = 17,              // Baranya
	Zala = 18,                 // Zala
	Vas = 19,                  // Vas
}

impl fmt::Display for County {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl TryFrom<u8> for County {
	type Error = anyhow::Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		let res = match value {
			0 => County::NoResponse,
			1 => County::Pest,
			2 => County::Nograd,
			3 => County::Heves,
			4 => County::JaszNagykunSzolnok,
			5 => County::BacsKiskun,
			6 => County::Fejer,
			7 => County::KomaromEsztergom,
			8 => County::Borsod,
			9 => County::HajduBihar,
			10 => County::Bekes,
			11 => County::Csongrad,
			12 => County::Tolna,
			13 => County::Somogy,
			14 => County::Veszprem,
			15 => County::GyorMosonSopron,
			16 => County::SzabolcsSzatmarBereg,
			17 => County::Baranya,
			18 => County::Zala,
			19 => County::Vas,
			_ => bail!("Invalid county number"),
		};
		Ok(res)
	}
}

impl FromStr for County {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"NoResponse" => Ok(County::NoResponse),
			"Pest" => Ok(County::Pest),
			"Nograd" => Ok(County::Nograd),
			"Heves" => Ok(County::Heves),
			"JaszNagykunSzolnok" => Ok(County::JaszNagykunSzolnok),
			"BacsKiskun" => Ok(County::BacsKiskun),
			"Fejer" => Ok(County::Fejer),
			"KomaromEsztergom" => Ok(County::KomaromEsztergom),
			"Borsod" => Ok(County::Borsod),
			"HajduBihar" => Ok(County::HajduBihar),
			"Bekes" => Ok(County::Bekes),
			"Csongrad" => Ok(County::Csongrad),
			"Tolna" => Ok(County::Tolna),
			"Somogy" => Ok(County::Somogy),
			"Veszprem" => Ok(County::Veszprem),
			"GyorMosonSopron" => Ok(County::GyorMosonSopron),
			"SzabolcsSzatmarBereg" => Ok(County::SzabolcsSzatmarBereg),
			"Baranya" => Ok(County::Baranya),
			"Zala" => Ok(County::Zala),
			"Vas" => Ok(County::Vas),
			_ => bail!("Invalid county name"),
		}
	}
}
pub(crate) fn available_serialize<S>(
	counties: &Option<AvailableAreas>,
	s: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let counties = match counties {
		// should return with error
		None => {
			return Err(serde::ser::Error::custom(
				"Serialization error: No hashset available",
			))
		}
		Some(ss) => {
			if ss.areas.is_empty() {
				return s.serialize_str("000000");
			} else {
				ss
			}
		}
	};
	// there might be more efficient methods than copying but this works for now
	let res = counties.areas.iter().map(|&county| county as i32).collect();
	s.serialize_str(&encode_available_areas(res))
}
pub fn decode_available_areas(available: i32) -> Vec<i32> {
	let mut res = Vec::new();
	for i in 1..=30 {
		if (available & (1 << (i - 1))) != 0 {
			res.push(i);
		}
	}
	res
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

	to_hex_with_length(&available_bytes, 6)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn county_serialize() {
		let decoded = decode_available_areas(i32::from_str_radix("07FFFF", 16).unwrap());
		assert_eq!(
			decoded,
			vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
		);
		assert_eq!(
			encode_available_areas(vec![
				1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18, 19
			],),
			"077FFF"
		)
	}
}