use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::HashesInterface;
use serde::{Serialize, Serializer};

// todo remove pub?
#[derive(Debug, Clone)]
pub struct GameState {
	pub state: i32,
	pub gameround: i32,
	pub phase: i32,
}

impl GameState {
	pub(crate) async fn set_gamestate(
		tmppool: &RedisPool,
		game_id: u32,
		state: GameState,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state:game_state", game_id),
				[
					("state", state.state),
					("game_round", state.gameround),
					("phase", state.phase),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_gamestate(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<GameState, anyhow::Error> {
		let res: HashMap<String, i32> = tmppool
			.hgetall(format!("games:{}:triviador_state:game_state", game_id))
			.await?;

		Ok(GameState {
			state: *res.get("state").unwrap(),
			gameround: *res.get("game_round").unwrap(),
			phase: *res.get("phase").unwrap(),
		})
	}
}

impl Serialize for GameState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{},{}", self.state, self.gameround, self.phase);

		serializer.serialize_str(&s)
	}
}
