use std::collections::{HashMap, HashSet};

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tracing::warn;

use crate::triviador::areas::Area;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::county::{Cmd, County};
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
					last_player: 0,
					next_player: 0,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: "0,0,0".to_string(),
				selection: "000000".to_string(),
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
	// pub async fn announcement_stage(
	// 	tmppool: &RedisPool,
	// 	game_id: u32,
	// ) -> Result<u8, anyhow::Error> {
	// 	let res = GameState::set_gamestate(
	// 		tmppool,
	// 		game_id,
	// 		GameState {
	// 			state: 1,
	// 			// these do not have to be set
	// 			gameround: 0,
	// 			phase: 0,
	// 		},
	// 	)
	// 	.await?;
	// 	Ok(res)
	// }

	/// Modifies a triviador game's state to area choosing stage
	///
	/// Sets the game's `phase` to 1, `roundinfo` to {`last_player`: 1, `next_player`: 1}, available
	/// areas to all counties
	pub async fn player_select_stage(
		tmppool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
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
				last_player: game_player_id,
				next_player: game_player_id,
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
		AvailableAreas::pop_county(tmppool, game_id, County::try_from(selected_area)?).await?;

		Bases::add_base(
			tmppool,
			game_id,
			PlayerNames::try_from(game_player_id)?,
			Base::new(selected_area),
		)
		.await?;

		Area::base_selected(
			tmppool,
			game_id,
			game_player_id,
			County::try_from(selected_area)?,
		)
		.await?;

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
		scores[game_player_id as usize - 1] += 1000;
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
