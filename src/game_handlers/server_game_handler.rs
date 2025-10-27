use sqlx::PgPool;
use tokio_stream::StreamExt;
use tracing::info;

use super::s_game::GamePlayerInfo;
use crate::app::{ListenPlayerChannel, ServerCommandChannel};
use crate::emulator::Emulator;
use crate::game_handlers::s_game::{SGame, SGamePlayerInfo};
use crate::triviador::game::{SharedTrivGame, TriviadorGame};
use crate::triviador::game_player_data::{GamePlayerData, PlayerName};
use crate::triviador::player_info::PlayerInfo;
use crate::triviador::triviador_state::GamePlayerChannels;

pub(crate) struct ServerGameHandler {}

// todo I hate this bad code but I have better things to do
impl ServerGameHandler {
	// todo merge these two together
	pub async fn new_friendly(
		player_channel: ListenPlayerChannel,
		command_channel: ServerCommandChannel,
		game_id: u32,
		db: PgPool,
	) {
		let players = PlayerInfo {
			p1_name: "xrtxn".to_string(),
			p2_name: "null".to_string(),
			p3_name: "null".to_string(),
			pd1: GamePlayerData::emulate(),
			pd2: GamePlayerData::new_bot(),
			pd3: GamePlayerData::new_bot(),
			you: "1,2,3".to_string(),
			game_id,
			room: "1".to_string(),
			rules: "0,0".to_string(),
		};

		let game = SharedTrivGame::new(TriviadorGame::new_game(players.clone(), db));
		// todo check
		let mut server_game_players = GamePlayerInfo::new();
		if players.pd1.is_bot() {
			server_game_players.add(PlayerName::Player1, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player1, SGamePlayerInfo::new(true));
		}
		if players.pd2.is_bot() {
			server_game_players.add(PlayerName::Player2, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player2, SGamePlayerInfo::new(true));
		}
		if players.pd3.is_bot() {
			server_game_players.add(PlayerName::Player3, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player3, SGamePlayerInfo::new(true));
		}

		// initial setup
		let mut server_game = SGame::new(game.arc_clone(), server_game_players.clone());

		let channels = GamePlayerChannels {
			xml_channel: player_channel.clone(),
			command_channel: command_channel.clone(),
		};

		let mut iter = server_game_players.players_with_info_stream();
		while let Some((player, info)) = iter.next().await {
			game.write().await.utils.add(*player, info.clone());
			if info.is_player() {
				game.write()
					.await
					.utils
					.get_player_mut(player)
					.unwrap()
					.set_channels(Some(channels.clone()));
			}
		}
		server_game.handle_all().await;
		info!("Game ended");

		// tokio::time::sleep(std::time::Duration::from_secs(15)).await;
		player_channel.clear_rx();
		command_channel.clear_rx();
	}

	pub async fn new_friendly_with_players(
		player_channel: ListenPlayerChannel,
		command_channel: ServerCommandChannel,
		players: PlayerInfo,
		game_id: u32,
		db: PgPool,
	) {
		let game = SharedTrivGame::new(TriviadorGame::new_game(players.clone(), db));
		let mut server_game_players = GamePlayerInfo::new();
		if players.pd1.is_bot() {
			server_game_players.add(PlayerName::Player1, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player1, SGamePlayerInfo::new(true));
		}
		if players.pd2.is_bot() {
			server_game_players.add(PlayerName::Player2, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player2, SGamePlayerInfo::new(true));
		}
		if players.pd3.is_bot() {
			server_game_players.add(PlayerName::Player3, SGamePlayerInfo::new(false));
		} else {
			server_game_players.add(PlayerName::Player3, SGamePlayerInfo::new(true));
		}

		// initial setup
		let mut server_game = SGame::new(game.arc_clone(), server_game_players.clone());

		let channels = GamePlayerChannels {
			xml_channel: player_channel.clone(),
			command_channel: command_channel.clone(),
		};

		let mut iter = server_game_players.players_with_info_stream();
		while let Some((player, info)) = iter.next().await {
			game.write().await.utils.add(*player, info.clone());
			if info.is_player() {
				game.write()
					.await
					.utils
					.get_player_mut(player)
					.unwrap()
					.set_channels(Some(channels.clone()));
			}
		}
		server_game.handle_all().await;
		info!("Game ended");

		// tokio::time::sleep(std::time::Duration::from_secs(15)).await;
		player_channel.clear_rx();
		command_channel.clear_rx();
	}
}
