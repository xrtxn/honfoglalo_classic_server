use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
// todo find out what this is
pub struct RoundInfo {
	pub lpnum: i32,
	pub next_player: i32,
}

impl RoundInfo {
	pub(crate) async fn set_roundinfo(
		tmppool: &RedisPool,
		game_id: u32,
		round_info: RoundInfo,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = tmppool
			.hset(
				format!("games:{}:triviador_state:round_info", game_id),
				[
					("lpnum", round_info.lpnum),
					("next_player", round_info.next_player),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_roundinfo(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<RoundInfo, anyhow::Error> {
		let res: HashMap<String, i32> = tmppool
			.hgetall(format!("games:{}:triviador_state:round_info", game_id))
			.await?;

		Ok(RoundInfo {
			lpnum: *res.get("lpnum").unwrap(),
			next_player: *res.get("next_player").unwrap(),
		})
	}
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{}", self.lpnum, self.next_player);

		serializer.serialize_str(&s)
	}
}
