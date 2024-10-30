use fred::clients::RedisPool;

use crate::game_handler::sgame::{SGame, SGamePlayer};
use crate::game_handler::PlayerType;
use crate::triviador::game::TriviadorGame;
use crate::triviador::game_player_data::GamePlayerData;
use crate::triviador::player_info::PlayerInfo;

pub(crate) struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(temp_pool: &RedisPool, game_id: u32) {
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

		let game = TriviadorGame::new_game(temp_pool, game_id, players)
			.await
			.unwrap();
		let players = game.players.unwrap();
		let server_game_players = vec![
			SGamePlayer::new(PlayerType::Player, players.pd1.id, 1),
			SGamePlayer::new(PlayerType::Bot, players.pd2.id, 2),
			SGamePlayer::new(PlayerType::Bot, players.pd3.id, 3),
		];

		// initial setup
		let mut server_game = SGame::new(server_game_players, game_id);
		loop {
			server_game.command(temp_pool).await;
			server_game.next();
		}
	}
}
