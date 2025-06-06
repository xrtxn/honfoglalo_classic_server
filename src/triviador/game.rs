use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_with::skip_serializing_none;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tracing::{error, trace};

use super::areas::Areas;
use super::available_area::AvailableAreas;
use super::game_player_data::PlayerName;
use super::player_points::PlayerPoints;
use crate::game_handlers::s_game::{GamePlayerInfo, SGamePlayerInfo};
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
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
	pub(crate) fn arc_clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}

	pub(crate) async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, TriviadorGame> {
		self.0.read().await
	}

	pub(crate) async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, TriviadorGame> {
		self.0.write().await
	}

	//todo better error handling
	pub(crate) async fn send_xml_channel(
		&self,
		player: &PlayerName,
		msg: String,
	) -> Result<(), flume::SendError<String>> {
		let game = self.read().await;
		let channel = game
			.utils
			.get_player(player)
			.unwrap()
			.get_player_channels()
			.clone();
		drop(game);
		channel.unwrap().xml_channel.send_message(msg).await
	}

	pub(crate) async fn recv_command_channel(
		&self,
		player: &PlayerName,
	) -> Result<ServerCommand, flume::RecvError> {
		let read_game = self.read().await;
		let channel = read_game
			.utils
			.get_player(player)
			.unwrap()
			.get_player_channels()
			.clone();
		drop(read_game);
		channel.unwrap().command_channel.recv_message().await
	}

	pub(crate) async fn wait_for_all_active(&self) {
		let utils = self.read().await.utils.clone();
		let mut iter = utils.active_players_stream();

		while let Some(player) = iter.next().await {
			// Wait for each player to respond
			match timeout(
				Duration::from_secs(10),
				wait_for_game_ready(self.arc_clone().borrow(), &player),
			)
			.await
			{
				Ok(_) => {
					// Player responded within timeout
				}
				Err(_) => {
					error!(
						"Player {:?} did not respond within 10 seconds timeout",
						player
					);
					trace!("State: {:?}", self.read().await.state.game_state);
				}
			}
		}
	}

	//todo clean up this function
	pub(crate) async fn wait_for_players(&self, players: Vec<PlayerName>) {
		let utils = self.read().await.utils.clone();

		let mut iter = tokio_stream::iter(players).filter_map(|player| {
			if utils.get_player(&player).unwrap().is_player() {
				Some(player)
			} else {
				None
			}
		});

		while let Some(player) = iter.next().await {
			// Wait for each player to respond
			match timeout(
				Duration::from_secs(10),
				wait_for_game_ready(self.arc_clone().borrow(), &player),
			)
			.await
			{
				Ok(_) => {
					// Player responded within timeout
				}
				Err(_) => {
					error!(
						"Player {:?} did not respond within 10 seconds timeout",
						player
					);
					trace!("State: {:?}", self.read().await.state.game_state);
				}
			}
		}
	}

	pub(crate) async fn send_to_all_active(&self) {
		// this avoids a deadlock
		let utils = self.read().await.utils.clone();
		let mut iter = utils.active_with_info_stream();
		while let Some((player, _)) = iter.next().await {
			send_player_commongame(self.arc_clone().borrow(), player).await;
			trace!("send_to_all_active: {:?}", player);
		}
	}

	// todo only return a stream
	pub(crate) async fn action_players(&self) -> HashMap<PlayerName, SGamePlayerInfo> {
		let game_reader = self.read().await;
		let mut players: HashMap<PlayerName, SGamePlayerInfo> = HashMap::with_capacity(2);
		let round_info = &game_reader.state.round_info;
		let active_player = round_info.active_player;
		players.insert(
			round_info.active_player,
			game_reader
				.utils
				.get_player(&round_info.active_player)
				.unwrap()
				.clone(),
		);
		game_reader.utils.get_player(&active_player);
		if let Some(player) = round_info.attacked_player {
			players.insert(
				player,
				game_reader.utils.get_player(&player).unwrap().clone(),
			);
		}
		players
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
#[serde(rename = "ROOT")]
pub struct TriviadorGame {
	#[serde(rename = "STATE")]
	pub(crate) state: TriviadorState,
	#[serde(rename = "PLAYERS")]
	pub(crate) players: Option<PlayerInfo>,
	#[serde(rename = "CMD")]
	pub(crate) cmd: Option<Cmd>,
	#[serde(skip)]
	pub(crate) utils: GamePlayerInfo,
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
					active_player: PlayerName::Nobody,
					attacked_player: None,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: PlayerPoints::new(),
				selection: Selection::new(),
				base_info: Bases::all_available(),
				areas_info: Areas::new(),
				available_areas: AvailableAreas::new(),
				used_helps: "0".to_string(),
				fill_round_winners: "".to_string(),
				room_type: None,
				shield_mission: None,
				war_order: None,
				active_player: None,
				eliminated_players: vec![],
			},
			players: Some(player_info),
			cmd: None,
			utils: GamePlayerInfo::new(),
		}
	}

	pub(crate) async fn add_fill_round_winner(&mut self, winner: Option<PlayerName>) {
		self.state
			.fill_round_winners
			.push_str(winner.unwrap().to_string().as_str());
	}
}
