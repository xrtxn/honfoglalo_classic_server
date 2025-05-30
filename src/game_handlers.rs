use tracing::error;

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

/// Waits for the game to be ready for a player by listening for a `Ready` command
/// Uses read lock
pub(crate) async fn wait_for_game_ready(receiver: &SharedTrivGame, player: &PlayerName) {
	let readgame = receiver.read().await;
	let utils = readgame.utils.clone();
	drop(readgame);
	if let Some(channels) = &utils.get_player(player).unwrap().get_player_channels() {
		match channels.command_channel.recv_message().await {
			Ok(ServerCommand::Ready) => {}
			Ok(_) => error!("Incorrect server command when waiting for game ready"),
			Err(e) => error!("Recv error when waiting for game ready: {}", e),
		}
	} else {
		error!("Channels not found for player");
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
