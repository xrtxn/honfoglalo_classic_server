use fred::prelude::RedisPool;

use crate::triviador::{GameState, TriviadorGame};
use crate::users::User;

pub struct ServerGameHandler {}

enum Players {
	Player1,
	Player2,
	Player3,
}

impl ServerGameHandler {
	pub async fn new_friendly(tmppool: &RedisPool, game_id: u32) {
		let game = TriviadorGame::new_game(tmppool, game_id).await.unwrap();
		let player_id = game.players.pd1.id;
		println!("{:#?}", game.players);
		// todo in the future, use pub/sub
		while !User::get_listen_state(tmppool, player_id).await.unwrap() {
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
		}

		let gamestate = GameState::get_gamestate(tmppool, game_id).await.unwrap();

		match gamestate.state {
			11 => {
				TriviadorGame::announcement(tmppool, game_id).await.unwrap();
			}
			1 => match gamestate.phase {
				0 => {
					TriviadorGame::choose_area(tmppool, game_id).await.unwrap();
				}
				1 => {
					todo!()
				}
				_ => {
					todo!()
				}
			},
			_ => {
				todo!()
			}
		}
		let xml = quick_xml::se::to_string(
			&TriviadorGame::get_triviador(tmppool, game_id)
				.await
				.unwrap(),
		)
		.unwrap();
		User::push_listen_queue(tmppool, player_id, &xml)
			.await
			.unwrap();
	}
}
