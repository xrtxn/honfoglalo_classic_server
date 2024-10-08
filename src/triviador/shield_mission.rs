use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct ShieldMission {
	pub shieldmission: i32,
	pub shieldmission_rt: i32,
}

impl ShieldMission {
	pub(crate) async fn set_shield_mission(
		temp_pool: &RedisPool,
		game_id: u32,
		mission: ShieldMission,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool
			.hset(
				format!("games:{}:triviador_state:shield_mission", game_id),
				[
					("shieldmission", mission.shieldmission),
					("shieldmission_rt", mission.shieldmission_rt),
				],
			)
			.await?;
		Ok(res)
	}

	pub(crate) async fn get_shield_mission(
		temp_pool: &RedisPool,
		game_id: u32,
		// todo this may be simplified
	) -> Result<Option<ShieldMission>, anyhow::Error> {
		let res: HashMap<String, i32> = temp_pool
			.hgetall(format!("games:{}:triviador_state:shield_mission", game_id))
			.await?;
		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(ShieldMission {
				shieldmission: *res.get("shieldmission").unwrap(),
				shieldmission_rt: *res.get("shieldmission_rt").unwrap(),
			}))
		}
	}
}

impl Serialize for ShieldMission {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// hexadecimal
		let s = format!("{:X},{:X}", self.shieldmission, self.shieldmission_rt);

		serializer.serialize_str(&s)
	}
}
