use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};
use serde_with::skip_serializing_none;

use crate::triviador::county::*;

#[derive(Debug)]
struct PlayerData {
	points: u16,
	chat_state: u8,
	is_connected: bool,
	// triviador.Game line 1748
	base: Base,
	selection: u8,
}

#[derive(PartialEq, Debug)]
struct Base {
	base_id: u8,
	towers_destroyed: u8,
}

impl Base {
	pub fn new_tower_destroyed(&mut self) {
		todo!();
		self.towers_destroyed += 1;
	}
}

impl Serialize for Base {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let base_part = self.towers_destroyed << 6;
		let hex = format!("{:02x}", self.base_id + base_part);
		serializer.serialize_str(&hex)
	}
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum PlayerNames {
	Player1,
	Player2,
	Player3,
}

#[derive(Clone, Debug)]
struct Area {
	owner: u8,
	is_fortress: bool,
	value: AreaValue,
}

impl Serialize for Area {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut ac = self.owner;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		let hex = format!("{:02x}", ac);
		serializer.serialize_str(&hex)
	}
}

#[derive(Serialize, Clone, Debug)]
// todo use the names instead of values
enum AreaValue {
	_1000 = 1,
	_400 = 2,
	_300 = 3,
	_200 = 4,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_ser_tokens, Token};

	use super::*;
	#[test]
	fn base_test() {
		let base = &Base {
			base_id: 2,
			towers_destroyed: 2,
		};
		assert_ser_tokens(&base, &[Token::String("82")]);

		let base = &Base {
			base_id: 8,
			towers_destroyed: 0,
		};
		assert_ser_tokens(&base, &[Token::String("08")]);
	}

	#[test]
	fn area_test() {
		let area = Area {
			owner: 1,
			is_fortress: false,
			value: AreaValue::_200,
		};

		assert_ser_tokens(&area, &[Token::String("41")]);
		let area = Area {
			owner: 3,
			is_fortress: false,
			value: AreaValue::_1000,
		};

		assert_ser_tokens(&area, &[Token::String("13")]);
	}

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

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
#[serde(rename = "ROOT")]
pub struct TriviadorGame {
	#[serde(rename = "STATE")]
	pub state: TriviadorState,
	#[serde(rename = "PLAYERS")]
	pub players: PlayerInfo,
	#[serde(rename = "CMD")]
	pub cmd: Option<Cmd>,
}

impl TriviadorGame {
	pub async fn new_game(tmppool: &RedisPool, game_id: u32) -> Result<TriviadorGame, RedisError> {
		let game = TriviadorGame {
			state: TriviadorState {
				map_name: "MAP_WD".to_string(),
				game_state: GameState {
					state: 11,
					gameround: 0,
					phase: 0,
				},
				round_info: RoundInfo {
					lpnum: 0,
					next_player: 0,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: "0,0,0".to_string(),
				selection: "000000".to_string(),
				base_info: "000000".to_string(),
				area_num: "0000000000000000000000000000000000000000".to_string(),
				available_areas: Some(AvailableAreas {
					areas: HashSet::new(),
				}),
				used_helps: "0".to_string(),
				room_type: None,
				shield_mission: None,
				war: None,
			},
			players: PlayerInfo {
				p1: "xrtxn".to_string(),
				p2: "null".to_string(),
				p3: "null".to_string(),
				pd1: "-1,14000,15,1,0,ar,1,,0".to_string(),
				pd2: "-1,14000,15,1,0,hu,1,,8".to_string(),
				pd3: "-1,14000,15,1,0,ar,1,,6".to_string(),
				you: "1,2,3".to_string(),
				game_id: "1".to_string(),
				room: "1".to_string(),
				rules: "0,0".to_string(),
			},
			cmd: None,
		};

		Self::set_triviador(tmppool, game_id, game.clone()).await?;
		Ok(game)
	}

	pub async fn set_triviador(
		tmppool: &RedisPool,
		game_id: u32,
		game: TriviadorGame,
	) -> Result<u8, RedisError> {
		{
			dbg!(0);
			let mut res = TriviadorState::set_triviador_state(tmppool, game_id, game.state).await?;
			dbg!(1);
			res += PlayerInfo::set_info(tmppool, game_id, game.players).await?;
			dbg!(2);
			if game.cmd.is_some() {
				res += Cmd::set_cmd(tmppool, game_id, game.cmd.unwrap()).await?;
				dbg!(3);
			}
			Ok(res)
		}
	}
	pub(crate) async fn get_triviador(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorGame, RedisError> {
		let _: HashMap<String, String> = tmppool.hgetall(format!("games:{}", game_id)).await?;
		let state = TriviadorState::get_triviador_state(tmppool, game_id).await?;
		let players = PlayerInfo::get_info(tmppool, game_id).await?;
		let cmd = Cmd::get_cmd(tmppool, game_id).await?;
		Ok(TriviadorGame {
			state,
			players,
			cmd,
		})
	}

	pub async fn announcement(tmppool: &RedisPool, game_id: u32) -> Result<u8, RedisError> {
		let res = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 1,
				// these do not have to be set
				gameround: 0,
				phase: 0,
			},
		)
		.await?;
		Ok(res)
	}
	pub async fn choose_area(tmppool: &RedisPool, game_id: u32) -> Result<u8, RedisError> {
		let mut res: u8 = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 1,
			},
		)
		.await?;

		// triviador.state.game_state.phase = 1;
		res += RoundInfo::set_roundinfo(
			tmppool,
			game_id,
			RoundInfo {
				lpnum: 1,
				next_player: 1,
			},
		)
		.await?;

		AvailableAreas::set_available(tmppool, game_id, AvailableAreas::all_counties()).await?;

		// end
		res += Cmd::set_cmd(
			tmppool,
			game_id,
			Cmd {
				command: "SELECT".to_string(),
				available: Some(AvailableAreas::all_counties()),
				cmd_timeout: 10,
			},
		)
		.await?;
		Ok(res)
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
pub struct TriviadorState {
	#[serde(rename = "@SCR")]
	pub map_name: String,
	// todo flatten
	#[serde(rename = "@ST")]
	pub game_state: GameState,
	#[serde(rename = "@CP")]
	pub round_info: RoundInfo,
	#[serde(rename = "@HC")]
	// todo use serde flatten
	// numbers of players connected e.g. 1,2,3
	pub players_connected: String,
	#[serde(rename = "@CHS")]
	pub players_chat_state: String,
	#[serde(rename = "@PTS")]
	pub players_points: String,
	#[serde(rename = "@SEL")]
	pub selection: String,
	#[serde(rename = "@B")]
	pub base_info: String,
	#[serde(rename = "@A")]
	// used by: Math.floor(Util.StringVal(tag.A).length / 2);
	pub area_num: String,
	#[serde(rename = "@AA", serialize_with = "available_serialize")]
	pub available_areas: Option<AvailableAreas>,
	#[serde(rename = "@UH")]
	pub used_helps: String,
	// possibly unused
	#[serde(rename = "@RT")]
	pub room_type: Option<String>,
	#[serde(rename = "@SMSR")]
	pub shield_mission: Option<ShieldMission>,
	#[serde(rename = "@WO")]
	// war order and rounds
	pub war: Option<String>,
}

impl TriviadorState {
	pub(crate) async fn set_triviador_state(
		tmppool: &RedisPool,
		game_id: u32,
		state: TriviadorState,
	) -> Result<u8, RedisError> {
		let mut res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[
					("map_name", state.map_name),
					// set game state
					// set round info
					("players_connected", state.players_connected),
					("players_chat_state", state.players_chat_state),
					("players_points", state.players_points),
					("selection", state.selection),
					("base_info", state.base_info),
					("area_num", state.area_num),
					// set available areas
					("used_helps", state.used_helps),
					// set room type if not none,
					// set shield mission
					// set war if not none,
				],
			)
			.await?;

		res += GameState::set_gamestate(tmppool, game_id, state.game_state).await?;
		res += RoundInfo::set_roundinfo(tmppool, game_id, state.round_info).await?;
		if state.available_areas.is_some() {
			AvailableAreas::set_available(tmppool, game_id, state.available_areas.unwrap()).await?;
		}
		if state.room_type.is_some() {
			let ares: u8 = tmppool
				.hset(
					format!("games:{}:triviador_state", game_id),
					("room_type", state.room_type.unwrap()),
				)
				.await?;
			res += ares;
		}
		if state.shield_mission.is_some() {
			let ares =
				ShieldMission::set_shield_mission(tmppool, game_id, state.shield_mission.unwrap())
					.await?;
			res += ares;
		}
		if state.war.is_some() {
			let ares: u8 = tmppool
				.hset(
					format!("games:{}:triviador_state", game_id),
					("war", state.war),
				)
				.await?;
			res += ares;
		}
		Ok(res)
	}

	pub(crate) async fn get_triviador_state(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorState, RedisError> {
		let res: HashMap<String, String> = tmppool
			.hgetall(format!("games:{}:triviador_state", game_id))
			.await?;

		let game_state = GameState::get_gamestate(tmppool, game_id).await?;
		let round_info = RoundInfo::get_roundinfo(tmppool, game_id).await?;
		let available_areas = AvailableAreas::get_available(tmppool, game_id).await?;
		let shield_mission = ShieldMission::get_shield_mission(tmppool, game_id).await?;
		Ok(TriviadorState {
			map_name: res.get("map_name").unwrap().to_string(),
			game_state,
			round_info,
			players_connected: res.get("players_connected").unwrap().to_string(),
			players_chat_state: res.get("players_chat_state").unwrap().to_string(),
			players_points: res.get("players_points").unwrap().to_string(),
			selection: res.get("selection").unwrap().to_string(),
			base_info: res.get("base_info").unwrap().to_string(),
			area_num: res.get("area_num").unwrap().to_string(),
			available_areas,
			used_helps: res.get("used_helps").unwrap().to_string(),
			// todo
			room_type: res.get("room_type").cloned(),
			shield_mission,
			war: res.get("war").cloned(),
		})
	}
}

#[derive(Debug, Clone)]
pub struct AvailableAreas {
	pub areas: HashSet<County>,
}

impl AvailableAreas {
	pub async fn set_empty(tmppool: &RedisPool, game_id: u32) -> Result<u8, RedisError> {
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
	) -> Result<u8, RedisError> {
		{
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
	}
	pub(crate) async fn get_available(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<Option<AvailableAreas>, RedisError> {
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

	pub(crate) async fn pop_county(tmppool: &RedisPool, game_id: u32) -> Result<u8, RedisError> {
		let res: u8 = tmppool
			.lrem(
				format!("games:{}:triviador_state:available_areas", game_id),
				0,
				-1,
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

#[derive(Serialize, Debug, Clone)]
pub struct PlayerInfo {
	#[serde(rename = "@P1")]
	pub p1: String,
	#[serde(rename = "@P2")]
	pub p2: String,
	#[serde(rename = "@P3")]
	pub p3: String,
	#[serde(rename = "@PD1")]
	pub pd1: String,
	#[serde(rename = "@PD2")]
	pub pd2: String,
	#[serde(rename = "@PD3")]
	pub pd3: String,
	#[serde(rename = "@YOU")]
	pub you: String,
	#[serde(rename = "@GAMEID")]
	pub game_id: String,
	#[serde(rename = "@ROOM")]
	pub room: String,
	#[serde(rename = "@RULES")]
	// this gives an interesting twist, 2 area instead of 1
	// possible values 1,0
	pub rules: String,
}

impl PlayerInfo {
	pub async fn set_info(
		tmppool: &RedisPool,
		game_id: u32,
		info: PlayerInfo,
	) -> Result<u8, RedisError> {
		{
			let res: u8 = tmppool
				.hset(
					format!("games:{}:info", game_id),
					[
						("p1", info.p1),
						("p2", info.p2),
						("p3", info.p3),
						("pd1", info.pd1),
						("pd2", info.pd2),
						("pd3", info.pd3),
						("you", info.you),
						("game_id", info.game_id),
						("room", info.room),
						("rules", info.rules),
					],
				)
				.await?;
			Ok(res)
		}
	}
	pub(crate) async fn get_info(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<PlayerInfo, RedisError> {
		let res: HashMap<String, String> =
			tmppool.hgetall(format!("games:{}:info", game_id)).await?;
		let info = PlayerInfo {
			p1: res.get("p1").unwrap().to_string(),
			p2: res.get("p2").unwrap().to_string(),
			p3: res.get("p3").unwrap().to_string(),
			pd1: res.get("pd1").unwrap().to_string(),
			pd2: res.get("pd2").unwrap().to_string(),
			pd3: res.get("pd3").unwrap().to_string(),
			you: res.get("you").unwrap().to_string(),
			game_id: res.get("game_id").unwrap().to_string(),
			room: res.get("room").unwrap().to_string(),
			rules: res.get("rules").unwrap().to_string(),
		};
		Ok(info)
	}
}

pub mod county {
	use std::fmt;
	use std::str::FromStr;

	use anyhow::anyhow;

	use super::*;
	#[derive(Debug, Serialize, Clone)]
	pub struct Cmd {
		#[serde(rename = "@CMD")]
		pub command: String,
		#[serde(rename = "@AVAILABLE", serialize_with = "available_serialize")]
		pub available: Option<AvailableAreas>,
		#[serde(rename = "@TO")]
		// seconds for action
		pub cmd_timeout: u8,
	}

	impl Cmd {
		pub async fn set_cmd(
			tmppool: &RedisPool,
			game_id: u32,
			cmd: Cmd,
		) -> Result<u8, RedisError> {
			{
				let mut res: u8 = tmppool
					.hset(
						format!("games:{}:cmd", game_id),
						[
							("command", cmd.command),
							("cmd_timeout", cmd.cmd_timeout.to_string()),
						],
					)
					.await?;
				if cmd.available.is_some() {
					res += AvailableAreas::set_available(tmppool, game_id, cmd.available.unwrap())
						.await?;
				}
				Ok(res)
			}
		}
		pub(crate) async fn get_cmd(
			tmppool: &RedisPool,
			game_id: u32,
		) -> Result<Option<Cmd>, RedisError> {
			// todo account for none
			let res: HashMap<String, String> =
				tmppool.hgetall(format!("games:{}:cmd", game_id)).await?;

			// return if none
			if res.is_empty() {
				return Ok(None);
			}

			let available = AvailableAreas::get_available(tmppool, game_id).await?;

			Ok(Some(Cmd {
				command: res.get("command").unwrap().to_string(),
				available,
				cmd_timeout: res.get("cmd_timeout").unwrap().parse()?,
			}))
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
				_ => Err(anyhow!("Invalid county name")),
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
			// todo return with error
			None => todo!(),
			Some(ss) => {
				// may be empty
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
}

#[derive(Debug, Clone)]
pub struct GameState {
	pub state: i32,
	pub gameround: i32,
	pub phase: i32,
}

impl GameState {
	pub(crate) async fn set_gamestate(
		tmppool: &RedisPool,
		game_id: u32,
		state: GameState,
	) -> Result<u8, RedisError> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state:game_state", game_id),
				[
					("state", state.state),
					("game_round", state.gameround),
					("phase", state.phase),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_gamestate(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<GameState, RedisError> {
		let res: HashMap<String, i32> = tmppool
			.hgetall(format!("games:{}:triviador_state:game_state", game_id))
			.await?;

		Ok(GameState {
			state: *res.get("state").unwrap(),
			gameround: *res.get("game_round").unwrap(),
			phase: *res.get("phase").unwrap(),
		})
	}
}

impl Serialize for GameState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{},{}", self.state, self.gameround, self.phase);

		serializer.serialize_str(&s)
	}
}

#[derive(Debug, Clone)]
// todo find out what this is
pub struct RoundInfo {
	pub lpnum: i32,
	pub next_player: i32,
}

impl RoundInfo {
	pub(crate) async fn set_roundinfo(
		tmppool: &RedisPool,
		game_id: u32,
		round_info: RoundInfo,
	) -> Result<u8, RedisError> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state:round_info", game_id),
				[
					("lpnum", round_info.lpnum),
					("next_player", round_info.next_player),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_roundinfo(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<RoundInfo, RedisError> {
		let res: HashMap<String, i32> = tmppool
			.hgetall(format!("games:{}:triviador_state:round_info", game_id))
			.await?;

		Ok(RoundInfo {
			lpnum: *res.get("lpnum").unwrap(),
			next_player: *res.get("next_player").unwrap(),
		})
	}
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{}", self.lpnum, self.next_player);

		serializer.serialize_str(&s)
	}
}

#[derive(Debug, Clone)]
pub struct ShieldMission {
	pub shieldmission: i32,
	pub shieldmission_rt: i32,
}

impl ShieldMission {
	pub(crate) async fn set_shield_mission(
		tmppool: &RedisPool,
		game_id: u32,
		mission: ShieldMission,
	) -> Result<u8, RedisError> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state:shield_mission", game_id),
				[
					("shieldmission", mission.shieldmission),
					("shieldmission_rt", mission.shieldmission_rt),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_shield_mission(
		tmppool: &RedisPool,
		game_id: u32,
		// todo this may be simplified
	) -> Result<Option<ShieldMission>, RedisError> {
		let res: HashMap<String, i32> = tmppool
			.hgetall(format!("games:{}:triviador_state:shield_mission", game_id))
			.await?;
		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(ShieldMission {
				shieldmission: *res.get("shieldmission").unwrap(),
				shieldmission_rt: *res.get("shieldmission_rt").unwrap(),
			}))
		}
	}
}

impl Serialize for ShieldMission {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// hexadecimal
		let s = format!("{:X},{:X}", self.shieldmission, self.shieldmission_rt);

		serializer.serialize_str(&s)
	}
}

fn to_hex_with_length(bytes: &[u8], length: usize) -> String {
	let encoded = hex::encode(bytes);
	let trimmed = encoded.trim_start_matches('0');

	// Format the string to the desired length
	format!("{:0>width$}", trimmed, width = length).to_uppercase()
}
