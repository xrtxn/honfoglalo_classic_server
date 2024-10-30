use std::collections::{HashMap, HashSet};

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tracing::warn;

use crate::triviador::areas::Area;
use crate::triviador::bases::Bases;
use crate::triviador::cmd::Cmd;
use crate::triviador::selection::Selection;
use crate::triviador::{
	available_area::AvailableAreas, game_state::GameState, player_info::PlayerInfo,
	round_info::RoundInfo, triviador_state::TriviadorState,
};

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
		temp_pool: &RedisPool,
		game_id: u32,
		player_info: PlayerInfo,
	) -> Result<TriviadorGame, anyhow::Error> {
		let game = TriviadorGame {
			state: TriviadorState {
				map_name: "MAP_WD".to_string(),
				game_state: GameState {
					state: 11,
					round: 0,
					phase: 0,
				},
				round_info: RoundInfo {
					last_player: 0,
					next_player: 0,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: "0,0,0".to_string(),
				selection: Selection::new(),
				base_info: Bases::all_available(),
				areas_info: Area::new(),
				available_areas: Some(AvailableAreas {
					areas: HashSet::new(),
				}),
				used_helps: "0".to_string(),
				room_type: None,
				shield_mission: None,
				war: None,
			},
			players: player_info,
			cmd: None,
		};

		Self::set_triviador(temp_pool, game_id, game.clone()).await?;
		Ok(game)
	}

	/// Sets a triviador game argument into redis
	pub async fn set_triviador(
		temp_pool: &RedisPool,
		game_id: u32,
		game: TriviadorGame,
	) -> Result<u8, anyhow::Error> {
		let mut res = TriviadorState::set_triviador_state(temp_pool, game_id, game.state).await?;
		res += PlayerInfo::set_info(temp_pool, game_id, game.players).await?;
		if game.cmd.is_some() {
			warn!("Setting triviador game CMD is NOT NULL, but it won't be set!")
		}
		Ok(res)
	}

	/// Gets a general triviador game argument from redis
	/// The cmd is always empty!
	pub(crate) async fn get_triviador(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<TriviadorGame, anyhow::Error> {
		let _: HashMap<String, String> = temp_pool.hgetall(format!("games:{}", game_id)).await?;
		let state = TriviadorState::get_triviador_state(temp_pool, game_id).await?;
		let players = PlayerInfo::get_info(temp_pool, game_id).await?;
		Ok(TriviadorGame {
			state,
			players,
			cmd: None,
		})
	}
}
