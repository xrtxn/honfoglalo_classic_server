use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
// todo find out what this is
pub struct RoundInfo {
	pub last_player: u8,
	pub next_player: u8,
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
					("last_player", round_info.last_player),
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
		let res: HashMap<String, u8> = tmppool
			.hgetall(format!("games:{}:triviador_state:round_info", game_id))
			.await?;

		Ok(RoundInfo {
			last_player: *res.get("last_player").unwrap(),
			next_player: *res.get("next_player").unwrap(),
		})
	}
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{}", self.last_player, self.next_player);

		serializer.serialize_str(&s)
	}
}
