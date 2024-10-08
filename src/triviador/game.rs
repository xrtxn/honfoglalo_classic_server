use std::collections::{HashMap, HashSet};

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tracing::warn;

use crate::triviador::areas::Area;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::selection::Selection;
use crate::triviador::{
	available_area::AvailableAreas, game_player_data::GamePlayerData,
	game_player_data::PlayerNames, game_state::GameState, player_info::PlayerInfo,
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
		// if game.cmd.is_some() {
		// 	res += Cmd::set_player_cmd(temp_pool, player_id, game.cmd.unwrap()).await?;
		// }
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
