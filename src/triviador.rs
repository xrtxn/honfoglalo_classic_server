use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use fred::clients::RedisPool;
use fred::prelude::*;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;
use tokio::try_join;
use tracing::warn;

use crate::triviador::county::*;
use crate::utils::split_string_n;

#[derive(Debug)]
struct PlayerData {
	points: u16,
	chat_state: u8,
	is_connected: bool,
	// triviador.Game line 1748
	base: Base,
	selection: u8,
}

#[derive(PartialEq, Debug, Clone)]
struct Base {
	base_id: u8,
	towers_destroyed: u8,
}

impl Base {
	pub fn serialize_to_hex(&self) -> String {
		let base_part = self.towers_destroyed << 6;
		crate::utils::to_hex_with_length(&[self.base_id + base_part], 2)
	}

	pub fn deserialize_from_hex(hex: &str) -> Result<Self, anyhow::Error> {
		let value = u8::from_str_radix(hex, 16)?;
		let towers_destroyed = value >> 6;
		let base_id = value & 0b0011_1111;
		Ok(Base {
			base_id,
			towers_destroyed,
		})
	}
}

impl Serialize for Base {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.serialize_to_hex())
	}
}

#[derive(Clone, Debug, PartialEq)]
struct Bases {
	every_base: HashMap<PlayerNames, Base>,
}

impl Bases {
	pub async fn get_redis(tmppool: &RedisPool, game_id: u32) -> Result<Self, anyhow::Error> {
		let res: String = tmppool
			.hget(format!("games:{}:triviador_state", game_id), "base_info")
			.await?;
		let rest = Self::deserialize_full(&res)?;
		Ok(rest)
	}

	pub async fn set_redis(
		tmppool: &RedisPool,
		game_id: u32,
		bases: Bases,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[("base_info", Bases::serialize_full(&bases)?)],
			)
			.await?;
		Ok(res)
	}

	pub fn serialize_full(bases: &Bases) -> Result<String, anyhow::Error> {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(6);
		for i in 1..4 {
			match bases.every_base.get(&PlayerNames::from(i)) {
				None => serialized.push_str("00"),
				Some(base) => serialized.push_str(&base.serialize_to_hex()),
			}
		}
		Ok(serialized)
	}

	pub fn deserialize_full(s: &str) -> Result<Self, anyhow::Error> {
		let vals = split_string_n(s, 2);
		let mut rest: HashMap<PlayerNames, Base> = HashMap::with_capacity(3);
		for (i, base_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't have Player0
				PlayerNames::from(i as u8 + 1),
				Base::deserialize_from_hex(base_str)?,
			);
		}
		Ok(Self { every_base: rest })
	}
}

impl Serialize for Bases {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&Bases::serialize_full(self).unwrap())
	}
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
enum PlayerNames {
	Player1 = 1,
	Player2 = 2,
	Player3 = 3,
}
impl From<u8> for PlayerNames {
	fn from(value: u8) -> Self {
		match value {
			1 => Self::Player1,
			2 => Self::Player2,
			3 => Self::Player3,
			_ => todo!(),
		}
	}
}

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
	pub owner: u8,
	pub is_fortress: bool,
	pub value: AreaValue,
}

impl Area {
	pub fn new() -> HashMap<County, Area> {
		HashMap::new()
	}

	pub fn serialize_to_hex(&self) -> String {
		let mut ac = self.owner;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		format!("{:02x}", ac)
	}

	pub fn deserialize_from_hex(hex: &str) -> Result<Self, anyhow::Error> {
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

	pub fn deserialize_full(s: String) -> Result<HashMap<County, Area>, anyhow::Error> {
		let vals = split_string_n(&s, 2);
		let mut rest: HashMap<County, Area> = HashMap::with_capacity(19);
		for (i, county_str) in vals.iter().enumerate() {
			rest.insert(
				// increase by 1 because we don't want the 0 value County
				County::from((i as u8) + 1),
				Area::deserialize_from_hex(county_str)?,
			);
		}
		Ok(rest)
	}

	pub async fn get_redis(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<HashMap<County, Area>, anyhow::Error> {
		let res: String = tmppool
			.hget(format!("games:{}:triviador_state", game_id), "area_num")
			.await?;
		let rest = Self::deserialize_full(res)?;
		Ok(rest)
	}

	pub fn serialize_full(set_counties: &HashMap<County, Area>) -> Result<String, anyhow::Error> {
		// later this may not be 38 for different countries
		let mut serialized = String::with_capacity(38);
		// start from 1 because we don't want the 0 value County
		for i in 1..20 {
			let county = County::from(i);
			let area = set_counties.get(&county);
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

	pub async fn set_redis(
		tmppool: &RedisPool,
		game_id: u32,
		serialized: String,
	) -> Result<u8, RedisError> {
		{
			let res: u8 = tmppool
				.hset(
					format!("games:{}:triviador_state", game_id),
					[("area_num", serialized)],
				)
				.await?;
			Ok(res)
		}
	}

	pub async fn change_area(
		tmppool: &RedisPool,
		game_id: u32,
		values: (County, Area),
	) -> Result<Option<Area>, anyhow::Error> {
		{
			let mut full = Self::get_redis(tmppool, game_id).await?;
			// return the replaced area info
			Ok(full.insert(values.0, values.1))
		}
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

pub(crate) fn areas_full_seralizer<S>(
	counties: &HashMap<County, Area>,
	s: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	s.serialize_str(&Area::serialize_full(counties).unwrap())
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
	/// Creates a new triviador game, returns it and also creates it into redis
	pub async fn new_game(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorGame, anyhow::Error> {
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
				// todo replace this
				base_info: Bases {
					every_base: HashMap::new(),
				},
				areas_info: Area::new(),
				available_areas: Some(AvailableAreas {
					areas: HashSet::new(),
				}),
				used_helps: "0".to_string(),
				room_type: None,
				shield_mission: None,
				war: None,
			},
			players: PlayerInfo {
				p1_name: "xrtxn".to_string(),
				p2_name: "null".to_string(),
				p3_name: "null".to_string(),
				pd1: GamePlayerData {
					id: 1,
					xp_points: 14000,
					xp_level: 15,
					game_count: 1,
					game_count_sr: 0,
					country_id: "hu".to_string(),
					castle_level: 1,
					custom_avatar: false,
					soldier: 0,
					act_league: 1,
				},
				pd2: GamePlayerData {
					id: -1,
					xp_points: 14000,
					xp_level: 15,
					game_count: 1,
					game_count_sr: 0,
					country_id: "hu".to_string(),
					castle_level: 1,
					custom_avatar: false,
					soldier: 8,
					act_league: 1,
				},
				pd3: GamePlayerData {
					id: -1,
					xp_points: 14000,
					xp_level: 15,
					game_count: 1,
					game_count_sr: 0,
					country_id: "hu".to_string(),
					castle_level: 1,
					custom_avatar: false,
					soldier: 6,
					act_league: 1,
				},
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

	/// Sets a triviador game argument into redis
	pub async fn set_triviador(
		tmppool: &RedisPool,
		game_id: u32,
		game: TriviadorGame,
	) -> Result<u8, anyhow::Error> {
		let mut res = TriviadorState::set_triviador_state(tmppool, game_id, game.state).await?;
		res += PlayerInfo::set_info(tmppool, game_id, game.players).await?;
		if game.cmd.is_some() {
			warn!("Setting triviador game CMD is NOT NULL, but it won't be set!")
		}
		// if game.cmd.is_some() {
		// 	res += Cmd::set_player_cmd(tmppool, player_id, game.cmd.unwrap()).await?;
		// }
		Ok(res)
	}

	/// Gets a general triviador game argument from redis
	/// The cmd is always empty!
	pub(crate) async fn get_triviador(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorGame, anyhow::Error> {
		let _: HashMap<String, String> = tmppool.hgetall(format!("games:{}", game_id)).await?;
		let state = TriviadorState::get_triviador_state(tmppool, game_id).await?;
		let players = PlayerInfo::get_info(tmppool, game_id).await?;
		// let cmd = Cmd::get_player_cmd(tmppool, game_id).await?;
		Ok(TriviadorGame {
			state,
			players,
			cmd: None,
		})
	}

	/// Modifies a triviador game's state to announcement stage
	/// Sets the game's state to 1
	pub async fn announcement_stage(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
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

	/// Modifies a triviador game's state to area choosing stage
	///
	/// Sets the game's `phase` to 1, `roundinfo` to {`lpnum`: 1, `next_player`: 1}, available areas
	/// to all counties
	pub async fn select_bases_stage(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
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

		Ok(res)
	}

	pub async fn base_selected_stage(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 3,
			},
		)
		.await?;

		Ok(res)
	}

	pub async fn new_base_selected(
		tmppool: &RedisPool,
		game_id: u32,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(tmppool, game_id, County::from(selected_area)).await?;

		let mut bases = Bases::get_redis(tmppool, game_id).await?;
		bases.every_base.insert(
			PlayerNames::from(game_player_id),
			Base {
				base_id: selected_area,
				towers_destroyed: 0,
			},
		);

		let _ = &Bases::set_redis(tmppool, game_id, bases).await?;
		let mut areas = Area::get_redis(tmppool, game_id).await?;
		areas.insert(
			County::from(selected_area),
			Area {
				owner: game_player_id,
				is_fortress: false,
				value: AreaValue::_1000,
			},
		);
		Area::set_redis(tmppool, game_id, Area::serialize_full(&areas)?).await?;
		let res = TriviadorState::set_field(
			tmppool,
			game_id,
			"selection",
			&Bases::serialize_full(&Bases::get_redis(tmppool, game_id).await?)?,
		)
		.await?;
		let scores = TriviadorState::get_field(tmppool, game_id, "players_points").await?;
		let mut scores: Vec<u16> = scores
			.split(',')
			.map(|x| x.parse::<u16>().unwrap())
			.collect();
		scores[0] += 1000;
		TriviadorState::set_field(
			tmppool,
			game_id,
			"players_points",
			&format!("{},{},{}", scores[0], scores[1], scores[2]),
		)
		.await?;
		Ok(res)
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
pub(crate) struct TriviadorState {
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
	pub base_info: Bases,
	#[serde(rename = "@A", serialize_with = "areas_full_seralizer")]
	// used by: Math.floor(Util.StringVal(tag.A).length / 2);
	pub areas_info: HashMap<County, Area>,
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
	) -> Result<u8, anyhow::Error> {
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
					// ("base_info", state.base_info),
					("area_num", Area::serialize_full(&state.areas_info).unwrap()),
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
		res += Bases::set_redis(tmppool, game_id, state.base_info).await?;
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
	) -> Result<TriviadorState, anyhow::Error> {
		let res: HashMap<String, String> = tmppool
			.hgetall(format!("games:{}:triviador_state", game_id))
			.await?;

		let game_state = GameState::get_gamestate(tmppool, game_id).await?;
		let round_info = RoundInfo::get_roundinfo(tmppool, game_id).await?;
		let base_info = Bases::get_redis(tmppool, game_id).await?;
		let available_areas = AvailableAreas::get_available(tmppool, game_id).await?;
		let shield_mission = ShieldMission::get_shield_mission(tmppool, game_id).await?;
		let areas_info = Area::get_redis(tmppool, game_id).await?;
		Ok(TriviadorState {
			map_name: res.get("map_name").unwrap().to_string(),
			game_state,
			round_info,
			players_connected: res.get("players_connected").unwrap().to_string(),
			players_chat_state: res.get("players_chat_state").unwrap().to_string(),
			players_points: res.get("players_points").unwrap().to_string(),
			selection: res.get("selection").unwrap().to_string(),
			base_info,
			areas_info,
			available_areas,
			used_helps: res.get("used_helps").unwrap().to_string(),
			// todo
			room_type: res.get("room_type").cloned(),
			shield_mission,
			war: res.get("war").cloned(),
		})
	}

	pub(crate) async fn set_field(
		tmppool: &RedisPool,
		game_id: u32,
		field: &str,
		value: &str,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[(field, value)],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_field(
		tmppool: &RedisPool,
		game_id: u32,
		field: &str,
	) -> Result<String, anyhow::Error> {
		let res: String = tmppool
			.hget(format!("games:{}:triviador_state", game_id), field)
			.await?;
		Ok(res)
	}
}

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

#[derive(Serialize, Deserialize, Debug)]
pub struct AreaSelection {
	#[serde(rename = "@AREA")]
	pub area: u8,
}

#[derive(Serialize, Debug, Clone)]
pub struct PlayerInfo {
	#[serde(rename = "@P1")]
	pub p1_name: String,
	#[serde(rename = "@P2")]
	pub p2_name: String,
	#[serde(rename = "@P3")]
	pub p3_name: String,
	#[serde(rename = "@PD1")]
	pub pd1: GamePlayerData,
	#[serde(rename = "@PD2")]
	pub pd2: GamePlayerData,
	#[serde(rename = "@PD3")]
	pub pd3: GamePlayerData,
	#[serde(rename = "@YOU")]
	pub you: String,
	#[serde(rename = "@GAMEID")]
	pub game_id: String,
	#[serde(rename = "@ROOM")]
	pub room: String,
	#[serde(rename = "@RULES")]
	// 1,0 possibly means quick game
	pub rules: String,
}

#[derive(Debug, Clone)]
pub struct GamePlayerData {
	pub id: i32,
	pub xp_points: i32,
	pub xp_level: i16,
	pub game_count: i32,
	// meaning?
	pub game_count_sr: i32,
	pub country_id: String,
	pub castle_level: i16,
	// this can be not existent with ,
	pub custom_avatar: bool,
	pub soldier: i16,
	pub act_league: i16,
}

impl GamePlayerData {
	pub async fn set_game_player_data(
		tmppool: &RedisPool,
		game_id: u32,
		game_player_number: i32,
		game_player_data: GamePlayerData,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:info", game_id),
				[(
					format!("pd{}", game_player_number),
					game_player_data.to_string(),
				)],
			)
			.await?;
		Ok(res)
	}
	pub async fn get_game_player_data(
		tmppool: &RedisPool,
		game_id: u32,
		player_id: i32,
	) -> Result<GamePlayerData, anyhow::Error> {
		let res: String = tmppool
			.hget(
				format!("games:{}:info", game_id),
				format!("pd{}", player_id),
			)
			.await?;
		let res: GamePlayerData = res.parse()?;
		Ok(res)
	}
}

impl FromStr for GamePlayerData {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parts: Vec<&str> = s.split(',').collect();

		let id = parts[0].parse::<i32>()?;
		let xp_points = parts[1].parse::<i32>()?;
		let xp_level = parts[2].parse::<i16>()?;
		let game_count = parts[3].parse::<i32>()?;
		let game_count_sr = parts[4].parse::<i32>()?;
		let country_id = parts[5].to_string();
		let castle_level = parts[6].parse::<i16>()?;

		// Handle custom_avatar as an optional boolean
		let custom_avatar = match parts[7] {
			"" => false, // Empty string represents a false value
			"true" => true,
			"false" => false,
			_ => bail!("Invalid custom_avatar value"),
		};

		let soldier = parts[8].parse::<i16>()?;
		let act_league = parts[9].parse::<i16>()?;

		Ok(GamePlayerData {
			id,
			xp_points,
			xp_level,
			game_count,
			game_count_sr,
			country_id,
			castle_level,
			custom_avatar,
			soldier,
			act_league,
		})
	}
}

impl fmt::Display for GamePlayerData {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let avatar = if self.custom_avatar {
			"todo_this_is_a_custom_avatar"
		} else {
			""
		};

		let str = format!(
			"{},{},{},{},{},{},{},{},{},{}",
			self.id,
			self.xp_points,
			self.xp_level,
			self.game_count,
			self.game_count_sr,
			self.country_id,
			self.castle_level,
			avatar,
			self.soldier,
			self.act_league
		);
		write!(f, "{}", str)
	}
}

impl Serialize for GamePlayerData {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.to_string().as_str())
	}
}

impl PlayerInfo {
	pub async fn set_info(
		tmppool: &RedisPool,
		game_id: u32,
		info: PlayerInfo,
	) -> Result<u8, anyhow::Error> {
		{
			let gpd_one_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 1, info.pd1);
			let gpd_two_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 2, info.pd2);
			let gpd_three_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 3, info.pd3);
			let info_fut = tmppool.hset::<u8, _, _>(
				format!("games:{}:info", game_id),
				[
					("p1_name", info.p1_name),
					("p2_name", info.p2_name),
					("p3_name", info.p3_name),
					("you", info.you),
					("game_id", info.game_id),
					("room", info.room),
					("rules", info.rules),
				],
			);
			let mut modified = 0;
			let res = try_join!(
				gpd_one_fut,
				gpd_two_fut,
				gpd_three_fut,
				info_fut.map_err(anyhow::Error::from)
			);
			match res {
				Ok(res) => {
					modified += res.0;
					modified += res.1;
					modified += res.2;
					modified += res.3;
				}
				Err(err) => bail!(err),
			}
			Ok(modified)
		}
	}
	pub async fn get_info(tmppool: &RedisPool, game_id: u32) -> Result<PlayerInfo, anyhow::Error> {
		let res: HashMap<String, String> =
			tmppool.hgetall(format!("games:{}:info", game_id)).await?;
		let pd1 = GamePlayerData::get_game_player_data(tmppool, game_id, 1).await?;
		let pd2 = GamePlayerData::get_game_player_data(tmppool, game_id, 2).await?;
		let pd3 = GamePlayerData::get_game_player_data(tmppool, game_id, 3).await?;
		Ok(PlayerInfo {
			p1_name: res.get("p1_name").unwrap().to_string(),
			p2_name: res.get("p2_name").unwrap().to_string(),
			p3_name: res.get("p3_name").unwrap().to_string(),
			pd1,
			pd2,
			pd3,
			you: res.get("you").unwrap().to_string(),
			game_id: res.get("game_id").unwrap().to_string(),
			room: res.get("room").unwrap().to_string(),
			rules: res.get("rules").unwrap().to_string(),
		})
	}
}

pub mod county {
	use std::fmt;
	use std::str::FromStr;

	use anyhow::bail;

	use super::*;
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
				let mut res: u8 = tmppool
					.hset(
						format!("users:{}:cmd", player_id),
						[
							("command", cmd.command),
							("cmd_timeout", cmd.timeout.to_string()),
						],
					)
					.await?;
				// if cmd.available.is_some() {
				// 	res += AvailableAreas::set_available(tmppool, game_id, cmd.available.unwrap())
				// 		.await?;
				// }
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

	impl From<u8> for County {
		fn from(value: u8) -> Self {
			match value {
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
				_ => todo!(),
			}
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
	) -> Result<u8, anyhow::Error> {
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
	) -> Result<GameState, anyhow::Error> {
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
	) -> Result<u8, anyhow::Error> {
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
	) -> Result<RoundInfo, anyhow::Error> {
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
	) -> Result<u8, anyhow::Error> {
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
	) -> Result<Option<ShieldMission>, anyhow::Error> {
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
		assert_eq!(base.serialize_to_hex(), "82");
		assert_eq!(Base::deserialize_from_hex("82").unwrap(), *base);

		let base = &Base {
			base_id: 8,
			towers_destroyed: 0,
		};
		assert_eq!(base.serialize_to_hex(), "08");
		assert_eq!(Base::deserialize_from_hex("08").unwrap(), *base);

		let s = "8C080B";
		let res = Bases::deserialize_full(s).unwrap();
		assert_eq!(
			Bases {
				every_base: HashMap::from([
					(
						PlayerNames::Player1,
						Base {
							base_id: 12,
							towers_destroyed: 2
						}
					),
					(
						PlayerNames::Player2,
						Base {
							base_id: 8,
							towers_destroyed: 0
						}
					),
					(
						PlayerNames::Player3,
						Base {
							base_id: 11,
							towers_destroyed: 0
						}
					)
				])
			},
			res
		);
	}

	#[test]
	fn area_test() {
		let area = Area {
			owner: 1,
			is_fortress: false,
			value: AreaValue::_200,
		};

		assert_ser_tokens(&area, &[Token::String("41")]);
		assert_eq!(Area::deserialize_from_hex("41").unwrap(), area);

		let area = Area {
			owner: 3,
			is_fortress: false,
			value: AreaValue::_1000,
		};

		assert_ser_tokens(&area, &[Token::String("13")]);
		let area = Area {
			owner: 0,
			is_fortress: false,
			value: AreaValue::Unoccupied,
		};

		assert_ser_tokens(&area, &[Token::String("00")]);
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

	#[test]
	fn full_area_serialize() {
		let res = Area::serialize_full(&HashMap::from([(
			County::SzabolcsSzatmarBereg,
			Area {
				owner: PlayerNames::Player3 as u8,
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
		let res =
			Area::deserialize_full("13434343434342424242434141421112414243".to_string()).unwrap();

		assert_eq!(
			*res.get(&County::Pest).unwrap(),
			Area {
				owner: 3,
				is_fortress: false,
				value: AreaValue::_1000
			}
		);

		assert_eq!(
			*res.get(&County::SzabolcsSzatmarBereg).unwrap(),
			Area {
				owner: 2,
				is_fortress: false,
				value: AreaValue::_1000
			}
		);
		assert_eq!(
			*res.get(&County::Baranya).unwrap(),
			Area {
				owner: 1,
				is_fortress: false,
				value: AreaValue::_200
			}
		)
	}
}
