use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::triviador::areas::areas_full_seralizer;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::Bases;
use crate::triviador::county::available_serialize;
use crate::triviador::county::County;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::shield_mission::ShieldMission;

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
pub(crate) struct TriviadorState {
	#[serde(rename = "@SCR")]
	pub map_name: String,
	#[serde(rename = "@ST")]
	pub game_state: GameState,
	#[serde(rename = "@CP")]
	pub round_info: RoundInfo,
	#[serde(rename = "@HC")]
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
