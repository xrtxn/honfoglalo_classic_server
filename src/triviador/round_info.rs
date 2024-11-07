use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct RoundInfo {
	pub mini_phase_num: u8,
	pub rel_player_id: u8,
	pub attacked_player: Option<u8>,
}

impl RoundInfo {
	pub(crate) async fn set_roundinfo(
		temp_pool: &RedisPool,
		game_id: u32,
		round_info: RoundInfo,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state:round_info", game_id),
				[
					("mini_phase_num", round_info.mini_phase_num),
					("rel_player_id", round_info.rel_player_id),
					("attacked_player", round_info.attacked_player.unwrap_or(255)),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_roundinfo(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<RoundInfo, anyhow::Error> {
		let res: HashMap<String, u8> = temp_pool
			.hgetall(format!("games:{}:triviador_state:round_info", game_id))
			.await?;

		let attacked = res
			.get("attacked_player")
			.and_then(|&v| if v == 255 { None } else { Some(v) });

		Ok(RoundInfo {
			mini_phase_num: *res.get("mini_phase_num").unwrap(),
			rel_player_id: *res.get("rel_player_id").unwrap(),
			attacked_player: attacked,
		})
	}

	pub(crate) async fn incr_mini_phase(
		temp_pool: &RedisPool,
		game_id: u32,
		by: u8,
	) -> Result<(), anyhow::Error> {
		let mut ri = RoundInfo::get_roundinfo(temp_pool, game_id).await?;
		ri.mini_phase_num += by;
		RoundInfo::set_roundinfo(temp_pool, game_id, ri).await?;
		Ok(())
	}
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{}", self.mini_phase_num, self.rel_player_id);

		serializer.serialize_str(&s)
	}
}
