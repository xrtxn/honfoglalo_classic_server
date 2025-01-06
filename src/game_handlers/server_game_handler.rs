use crate::app::{ServerCommandChannel, XmlPlayerChannel};
use crate::game_handlers::s_game::{SGame, SGamePlayer};
use crate::game_handlers::PlayerType;
use crate::triviador::game::{PlayerUtils, SharedTrivGame, TriviadorGame};
use crate::triviador::game_player_data::GamePlayerData;
use crate::triviador::player_info::PlayerInfo;
use crate::triviador::triviador_state::GamePlayerChannels;

pub(crate) struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(
		player_channel: XmlPlayerChannel,
		command_channel: ServerCommandChannel,
		game_id: u32,
	) {
		let players = PlayerInfo {
			p1_name: "xrtxn".to_string(),
			p2_name: "null".to_string(),
			p3_name: "null".to_string(),
			pd1: GamePlayerData::emu_player(),
			pd2: GamePlayerData::new_bot(),
			pd3: GamePlayerData::new_bot(),
			you: "1,2,3".to_string(),
			game_id,
			room: "1".to_string(),
			rules: "0,0".to_string(),
		};

		let game = SharedTrivGame::new(TriviadorGame::new_game(players));
		// todo check
		let players = game.read().await.players.clone().unwrap();
		let server_game_players = vec![
			SGamePlayer::new(PlayerType::Player, players.pd1.id, 1),
			SGamePlayer::new(PlayerType::Bot, players.pd2.id, 2),
			SGamePlayer::new(PlayerType::Bot, players.pd3.id, 3),
		];

		// initial setup
		let mut server_game = SGame::new(game.arc_clone(), server_game_players);

		let channels = GamePlayerChannels {
			xml_channel: player_channel.clone(),
			command_channel: command_channel.clone(),
		};

		game.write().await.utils.insert(
			server_game.players[0].clone(),
			PlayerUtils {
				cmd: None,
				channels: Some(channels),
			},
		);

		// todo this is a temporary solution
		loop {
			server_game
				// avoid cloning in the future
				.command()
				.await;
			server_game.next();
		}
	}
}
