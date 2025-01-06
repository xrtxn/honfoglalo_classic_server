use s_game::SGamePlayer;
use tracing::error;

use crate::triviador::game::SharedTrivGame;
use crate::users::ServerCommand;

pub(super) mod area_conquer_handler;
pub(super) mod base_handler;
pub(super) mod battle_handler;
pub(super) mod endscreen_handler;
pub(super) mod fill_remaining_handler;
pub(super) mod question_handler;
pub(super) mod s_game;
pub(super) mod server_game_handler;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PlayerType {
	Player,
	Bot,
}

pub(crate) async fn wait_for_game_ready(receiver: &SharedTrivGame, player: &SGamePlayer) {
	let readgame = receiver.read().await;
	let utils = readgame.utils.clone();
	drop(readgame);
	if let Some(player_utils) = utils.get(player) {
		if let Some(channels) = &player_utils.channels {
			// let command_channel = channels.command_channel.clone();
			match channels.command_channel.recv_message().await {
				Ok(ServerCommand::Ready) => {}
				Ok(_) => error!("Incorrect server command when waiting for game ready"),
				Err(e) => error!("Recv error when waiting for game ready: {}", e),
			}
		} else {
			error!("Channels not found for player");
		}
	} else {
		error!("Player utils not found");
	}
}

pub(crate) async fn send_player_commongame(game: &SharedTrivGame, player: &SGamePlayer) {
	// todo may be bad
	let mut commanded = game.read().await.clone();
	commanded.cmd = commanded.utils.get(player).unwrap().cmd.clone();
	let xml = quick_xml::se::to_string(&commanded).unwrap();
	game.send_xml_channel(player, xml).await.unwrap();
	if let Some(x) = game.write().await.utils.get_mut(player) {
		x.cmd = None;
	}
}
