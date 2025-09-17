use std::borrow::Borrow;
use std::sync::Arc;

use serde::Serialize;
use serde_with::skip_serializing_none;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::trace;

use super::areas::Areas;
use super::available_area::AvailableAreas;
use super::fill_round::FillRound;
use super::game_player_data::PlayerName;
use super::player_points::PlayerPoints;
use crate::game_handlers::s_game::GamePlayerInfo;
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

	pub(crate) async fn loop_recv_command(
		&self,
		player: &PlayerName,
		desired_command: ServerCommand,
	) -> anyhow::Result<ServerCommand> {
		trace!(
			"loop_recv_command for player: {:?}, desired_command: {:?}",
			player, desired_command
		);
		let timeout_duration = tokio::time::Duration::from_secs(15);

		match tokio::time::timeout(timeout_duration, async {
			let mut command = self.recv_command_channel(player).await?;
			while !command.variant_eq(&desired_command) {
				trace!(
					"Waiting for command {:?} for player: {:?}, received: {:?}",
					desired_command, player, command
				);
				command = self.recv_command_channel(player).await?;
			}
			Ok(command)
		})
		.await
		{
			Ok(Ok(command)) => Ok(command), // Inner future succeeded and returned Ok(command)
			Ok(Err(e)) => Err(e), // Inner future succeeded but returned an Err(e) from recv_command_channel
			Err(_) => Err(anyhow::anyhow!(
				"Timeout (10s) waiting for command {:?} for player: {:?}",
				desired_command,
				player
			)),
		}
	}

	pub(crate) async fn wait_for_all_active(&self) {
		let utils = self.read().await.utils.clone();
		let iter = utils.active_players_stream();

		futures::stream::StreamExt::for_each_concurrent(iter, None, |player| {
			let game = self.arc_clone();
			async move {
				wait_for_game_ready(game.borrow(), player).await;
			}
		})
		.await;
	}

	//todo clean up this function
	pub(crate) async fn wait_for_players(&self, players: Vec<PlayerName>) {
		let utils = self.read().await.utils.clone();

		let iter = tokio_stream::iter(players).filter_map(|player| {
			if utils.get_player(&player).unwrap().is_player() {
				Some(player)
			} else {
				None
			}
		});

		futures::stream::StreamExt::for_each_concurrent(iter, 2, |player| {
			let game = self.arc_clone();
			async move {
				wait_for_game_ready(game.borrow(), &player).await;
			}
		})
		.await;
	}

	//todo make async
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
	// pub(crate) async fn action_players(&self) -> HashMap<PlayerName, SGamePlayerInfo> {
	// 	let game_reader = self.read().await;
	// 	let mut players: HashMap<PlayerName, SGamePlayerInfo> = HashMap::with_capacity(2);
	// 	let round_info = &game_reader.state.round_info;
	// 	let active_player = round_info.active_player;
	// 	players.insert(
	// 		round_info.active_player,
	// 		game_reader
	// 			.utils
	// 			.get_player(&round_info.active_player)
	// 			.unwrap()
	// 			.clone(),
	// 	);
	// 	game_reader.utils.get_player(&active_player);
	// 	if let Some(player) = round_info.attacked_player {
	// 		players.insert(
	// 			player,
	// 			game_reader.utils.get_player(&player).unwrap().clone(),
	// 		);
	// 	}
	// 	players
	// }
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
	#[serde(skip)]
	pub(crate) db: PgPool,
}

impl TriviadorGame {
	/// Creates a new triviador game
	pub(crate) fn new_game(player_info: PlayerInfo, db: PgPool) -> TriviadorGame {
		TriviadorGame {
			state: TriviadorState {
				map_name: "MAP_WD".to_string(),
				game_state: GameState::loading_screen(),
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
				fill_round_winners: FillRound::new(),
				room_type: None,
				shield_mission: None,
				war_order: None,
				active_player: None,
				eliminated_players: vec![],
			},
			players: Some(player_info),
			cmd: None,
			utils: GamePlayerInfo::new(),
			db,
		}
	}
}
