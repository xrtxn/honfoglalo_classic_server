use std::time::Duration;

use tokio::time::timeout;
use tracing::{error, trace};

use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::users::ServerCommand;

pub(super) mod area_conquer_handler;
pub(super) mod base_handler;
pub(super) mod battle_handler;
pub(super) mod endscreen_handler;
pub(super) mod fill_remaining_handler;
pub(super) mod question_handler;
pub(super) mod s_game;
pub(super) mod server_game_handler;

//todo move this over to game.rs

/// Waits for the game to be ready for a player by listening for a `Ready` command
/// Uses read lock with 10s timeout
pub(crate) async fn wait_for_game_ready(receiver: &SharedTrivGame, player: &PlayerName) {
	let readgame = receiver.read().await;
	let utils = readgame.utils.clone();
	drop(readgame);
	if let Some(channels) = &utils.get_player(player).unwrap().get_player_channels() {
		let timeout_duration = Duration::from_secs(10);

		let result = timeout(timeout_duration, async {
			let mut command = channels
				.command_channel
				.recv_message()
				.await
				.expect("Failed to receive command");
			while !command.variant_eq(&ServerCommand::Ready) {
				command = channels
					.command_channel
					.recv_message()
					.await
					.expect("Failed to receive command");
			}
		})
		.await;

		if result.is_err() {
			error!(
				"Timeout waiting for Ready command from player: {:?}",
				player
			);
			trace!("State: {:?}", receiver.read().await.state.game_state);
		}
		// channels.command_channel.clear();
	} else {
		error!("Channels not found for player: {:?}", player);
	}
}

/// Sends the common game state to a player
/// Uses write lock
pub(crate) async fn send_player_commongame(game: &SharedTrivGame, player: &PlayerName) {
	let mut commanded = game.read().await.clone();
	commanded.cmd = commanded
		.utils
		.get_player(player)
		.unwrap()
		.get_cmd()
		.clone();
	let xml = quick_xml::se::to_string(&commanded).unwrap();
	game.send_xml_channel(player, xml).await.unwrap();
	drop(commanded);
	game.write()
		.await
		.utils
		.get_player_mut(player)
		.unwrap()
		.set_cmd(None);
}
