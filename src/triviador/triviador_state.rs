use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tracing::error;

use crate::triviador::areas::areas_full_serializer;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::Bases;
use crate::triviador::county::available_serialize;
use crate::triviador::county::County;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
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
	pub selection: Selection,
	#[serde(rename = "@B")]
	pub base_info: Bases,
	#[serde(rename = "@A", serialize_with = "areas_full_serializer")]
	pub areas_info: HashMap<County, Area>,
	#[serde(rename = "@AA", serialize_with = "available_serialize")]
	pub available_areas: Option<AvailableAreas>,
	#[serde(rename = "@UH")]
	pub used_helps: String,
	#[serde(rename = "@FAO")]
	pub fill_round: Option<i8>,
	// possibly unused
	#[serde(rename = "@RT")]
	pub room_type: Option<String>,
	#[serde(rename = "@SMSR")]
	pub shield_mission: Option<ShieldMission>,
	#[serde(rename = "@WO")]
	// war order and rounds
	pub war_order: Option<String>,
}

impl TriviadorState {
	pub(crate) async fn set_triviador_state(
		temp_pool: &RedisPool,
		game_id: u32,
		state: TriviadorState,
	) -> Result<u8, anyhow::Error> {
		let mut res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[
					("map_name", state.map_name),
					// set game state
					// set round info
					("players_connected", state.players_connected),
					("players_chat_state", state.players_chat_state),
					("players_points", state.players_points),
					// ("selection", state.selection),
					// ("base_info", state.base_info),
					("area_num", Area::serialize_full(&state.areas_info).unwrap()),
					// set available areas
					("fill_round", state.fill_round.unwrap_or(-1).to_string()),
					("used_helps", state.used_helps),
					// set room type if not none,
					// set shield mission
					// set war if not none,
				],
			)
			.await?;

		res += GameState::set_gamestate(temp_pool, game_id, state.game_state).await?;
		res += RoundInfo::set_roundinfo(temp_pool, game_id, state.round_info).await?;
		res += Selection::set_redis(temp_pool, game_id, state.selection).await?;
		res += Bases::set_redis(temp_pool, game_id, state.base_info).await?;
		if state.available_areas.is_some() {
			AvailableAreas::set_available(temp_pool, game_id, state.available_areas.unwrap())
				.await?;
		}
		if state.room_type.is_some() {
			let ares: u8 = temp_pool
				.hset(
					format!("games:{}:triviador_state", game_id),
					("room_type", state.room_type.unwrap()),
				)
				.await?;
			res += ares;
		}
		if state.shield_mission.is_some() {
			let ares = ShieldMission::set_shield_mission(
				temp_pool,
				game_id,
				state.shield_mission.unwrap(),
			)
			.await?;
			res += ares;
		}
		if state.war_order.is_some() {
			let ares: u8 = temp_pool
				.hset(
					format!("games:{}:triviador_state", game_id),
					("war_order", state.war_order),
				)
				.await?;
			res += ares;
		}
		Ok(res)
	}

	pub(crate) async fn get_triviador_state(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorState, anyhow::Error> {
		let res: HashMap<String, String> = temp_pool
			.hgetall(format!("games:{}:triviador_state", game_id))
			.await?;
		// todo very bad please fix
		let active_player: u8 = temp_pool
			.get(format!("games:{}:send_player", game_id.to_string()))
			.await
			.unwrap_or_else(|e| {
				error!("Error getting active player: {}", e);
				1
			});

		let game_state = GameState::get_gamestate(temp_pool, game_id).await?;
		let round_info = RoundInfo::get_roundinfo(temp_pool, game_id).await?;
		let selection = Selection::get_redis(temp_pool, game_id).await?;
		let base_info = Bases::get_redis(temp_pool, game_id).await?;

		// todo I dislike this
		let available_areas;
		if game_state.state == 1 {
			available_areas = AvailableAreas::get_available(temp_pool, game_id).await;
		} else {
			available_areas =
				AvailableAreas::get_limited_available(temp_pool, game_id, active_player).await;
		}
		// if the value is 0 set it to none
		let fill_round = res
			.get("fill_round")
			.and_then(|x| x.parse::<i8>().ok())
			.filter(|&x| x != -1);

		let shield_mission = ShieldMission::get_shield_mission(temp_pool, game_id).await?;
		let areas_info = Area::get_redis(temp_pool, game_id).await?;
		Ok(TriviadorState {
			map_name: res.get("map_name").unwrap().to_string(),
			game_state,
			round_info,
			players_connected: res.get("players_connected").unwrap().to_string(),
			players_chat_state: res.get("players_chat_state").unwrap().to_string(),
			players_points: res.get("players_points").unwrap().to_string(),
			selection,
			base_info,
			areas_info,
			available_areas,
			used_helps: res.get("used_helps").unwrap().to_string(),
			fill_round,
			room_type: res.get("room_type").cloned(),
			shield_mission,
			war_order: res.get("war_order").cloned(),
		})
	}

	pub(crate) async fn set_field(
		temp_pool: &RedisPool,
		game_id: u32,
		field: &str,
		value: &str,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state", game_id),
				[(field, value)],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_field(
		temp_pool: &RedisPool,
		game_id: u32,
		field: &str,
	) -> Result<String, anyhow::Error> {
		let res: String = temp_pool
			.hget(format!("games:{}:triviador_state", game_id), field)
			.await?;
		Ok(res)
	}

	pub(crate) async fn modify_scores(
		temp_pool: &RedisPool,
		game_id: u32,
		by: Vec<i16>,
	) -> Result<(), anyhow::Error> {
		let scores = Self::get_field(temp_pool, game_id, "players_points").await?;
		let mut scores: Vec<i16> = scores
			.split(',')
			.map(|x| x.parse::<i16>().unwrap())
			.collect();
		for (i, score) in scores.iter_mut().enumerate() {
			*score += by[i];
		}
		TriviadorState::set_field(
			temp_pool,
			game_id,
			"players_points",
			&format!("{},{},{}", scores[0], scores[1], scores[2]),
		)
		.await?;
		Ok(())
	}

	pub(crate) async fn modify_player_score(
		temp_pool: &RedisPool,
		game_id: u32,
		rel_id: u8,
		by: i16,
	) -> Result<(), anyhow::Error> {
		let scores = Self::get_field(temp_pool, game_id, "players_points").await?;
		let mut scores: Vec<i16> = scores
			.split(',')
			.map(|x| x.parse::<i16>().unwrap())
			.collect();
		for i in 0..3 {
			if i == rel_id {
				scores[i as usize] += by;
				break;
			}
		}
		TriviadorState::set_field(
			temp_pool,
			game_id,
			"players_points",
			&format!("{},{},{}", scores[0], scores[1], scores[2]),
		)
		.await?;
		Ok(())
	}
}
