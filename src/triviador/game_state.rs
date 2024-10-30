use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::HashesInterface;
use serde::{Serialize, Serializer};

// todo remove pub?
#[derive(Debug, Clone)]
pub(crate) struct GameState {
	pub state: u8,
	pub round: u8,
	pub phase: u8,
}

impl GameState {
	pub(crate) async fn incr_state(
		temp_pool: &RedisPool,
		game_id: u32,
		by: u8,
	) -> Result<(), anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.state += by;
		Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(())
	}
	pub(crate) async fn incr_round(
		temp_pool: &RedisPool,
		game_id: u32,
		by: u8,
	) -> Result<(), anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.round += by;
		Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(())
	}
	pub(crate) async fn incr_phase(
		temp_pool: &RedisPool,
		game_id: u32,
		by: u8,
	) -> Result<u8, anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.phase += by;
		let res = Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(res)
	}
	pub(crate) async fn set_state(
		temp_pool: &RedisPool,
		game_id: u32,
		state: u8,
	) -> Result<(), anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.state = state;
		Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(())
	}

	pub(crate) async fn set_round(
		temp_pool: &RedisPool,
		game_id: u32,
		round: u8,
	) -> Result<u8, anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.round = round;
		let res = Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(res)
	}
	pub(crate) async fn set_phase(
		temp_pool: &RedisPool,
		game_id: u32,
		phase: u8,
	) -> Result<u8, anyhow::Error> {
		let mut game_state = Self::get_gamestate(temp_pool, game_id).await?;
		game_state.phase = phase;
		let res = Self::set_gamestate(temp_pool, game_id, game_state).await?;
		Ok(res)
	}

	pub(crate) async fn set_gamestate(
		temp_pool: &RedisPool,
		game_id: u32,
		state: GameState,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state:game_state", game_id),
				[
					("state", state.state),
					("game_round", state.round),
					("phase", state.phase),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_gamestate(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<GameState, anyhow::Error> {
		let res: HashMap<String, u8> = temp_pool
			.hgetall(format!("games:{}:triviador_state:game_state", game_id))
			.await?;

		Ok(GameState {
			state: *res.get("state").unwrap(),
			round: *res.get("game_round").unwrap(),
			phase: *res.get("phase").unwrap(),
		})
	}
}

impl Serialize for GameState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{},{}", self.state, self.round, self.phase);

		serializer.serialize_str(&s)
	}
}
