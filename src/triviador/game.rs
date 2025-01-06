use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::Serialize;
use serde_with::skip_serializing_none;
use tokio::sync::RwLock;

use super::available_area::AvailableAreas;
use super::triviador_state::GamePlayerChannels;
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::bases::Bases;
use crate::triviador::cmd::Cmd;
use crate::triviador::selection::Selection;
use crate::triviador::{
	game_state::GameState, player_info::PlayerInfo, round_info::RoundInfo,
	triviador_state::TriviadorState,
};
use crate::users::ServerCommand;

pub struct SharedTrivGame(Arc<RwLock<TriviadorGame>>);

impl SharedTrivGame {
	pub fn new(game: TriviadorGame) -> Self {
		Self(Arc::new(RwLock::new(game)))
	}

	/// Convenience function for Arc clone
	pub fn arc_clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}

	pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, TriviadorGame> {
		self.0.read().await
	}

	pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, TriviadorGame> {
		self.0.write().await
	}

	pub async fn send_xml_channel(
		&self,
		player: &SGamePlayer,
		msg: String,
	) -> Result<(), flume::SendError<String>> {
		let game = self.read().await;
		let channel = game.utils.get(player).unwrap().channels.clone();
		drop(game);
		channel.unwrap().xml_channel.send_message(msg).await
	}

	pub(crate) async fn recv_command_channel(
		&self,
		player: &SGamePlayer,
	) -> Result<ServerCommand, flume::RecvError> {
		let game = self.read().await;
		let channel = game.utils.get(player).unwrap().channels.clone();
		drop(game);
		channel.unwrap().command_channel.recv_message().await
	}

	// todo consider meging these two functions
	pub(crate) async fn wait_for_all_players(&self, players: &[SGamePlayer]) {
		for player in players.iter().filter(|x| x.is_player()) {
			wait_for_game_ready(self.arc_clone().borrow(), player).await;
		}
	}
	pub(crate) async fn send_to_all_players(&self, players: &[SGamePlayer]) {
		for player in players.iter().filter(|x| x.is_player()) {
			send_player_commongame(self.arc_clone().borrow(), player).await;
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
	pub players: Option<PlayerInfo>,
	#[serde(rename = "CMD")]
	pub cmd: Option<Cmd>,
	#[serde(skip)]
	pub utils: HashMap<SGamePlayer, PlayerUtils>,
}

impl TriviadorGame {
	/// Creates a new triviador game
	pub(crate) fn new_game(player_info: PlayerInfo) -> TriviadorGame {
		TriviadorGame {
			state: TriviadorState {
				map_name: "MAP_WD".to_string(),
				game_state: GameState {
					state: 11,
					round: 0,
					phase: 0,
				},
				round_info: RoundInfo {
					mini_phase_num: 0,
					rel_player_id: 0,
					attacked_player: None,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: "0,0,0".to_string(),
				selection: Selection::new(),
				base_info: Bases::all_available(),
				areas_info: Area::new(),
				available_areas: AvailableAreas::new(),
				used_helps: "0".to_string(),
				fill_round_winners: "".to_string(),
				room_type: None,
				shield_mission: None,
				war_order: None,
				active_player: None,
			},
			players: Some(player_info),
			cmd: None,
			utils: HashMap::new(),
		}
	}

	pub(crate) async fn set_player_cmd(&mut self, player: &SGamePlayer, cmd: Option<Cmd>) {
		self.utils.get_mut(player).unwrap().cmd = cmd;
	}

	pub(crate) async fn add_fill_round_winner(&mut self, winner: u8) {
		self.state
			.fill_round_winners
			.push_str(winner.to_string().as_str());
	}
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerUtils {
	pub cmd: Option<Cmd>,
	pub channels: Option<GamePlayerChannels>,
}

impl PlayerUtils {
	pub fn new() -> Self {
		Self {
			cmd: None,
			channels: None,
		}
	}
}
