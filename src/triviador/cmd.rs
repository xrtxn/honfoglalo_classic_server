use std::collections::HashMap;

use fred::clients::RedisPool;
use fred::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::triviador::available_area::AvailableAreas;
use crate::triviador::county::available_serialize;

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct Cmd {
	#[serde(rename = "@CMD")]
	pub command: String,
	#[serde(rename = "@AVAILABLE", serialize_with = "available_serialize")]
	pub available: Option<AvailableAreas>,
	#[serde(rename = "@TO")]
	// seconds for action
	pub timeout: u8,
}

impl Cmd {
	pub fn select_command(available_areas: Option<AvailableAreas>, timeout: u8) -> Cmd {
		Cmd {
			command: "SELECT".to_string(),
			available: available_areas,
			timeout,
		}
	}
	pub fn answer_command(timeout: u8) -> Cmd {
		Cmd {
			command: "ANSWER".to_string(),
			available: None,
			timeout,
		}
	}

	pub async fn set_player_cmd(
		temp_pool: &RedisPool,
		player_id: i32,
		cmd: Cmd,
	) -> Result<u8, anyhow::Error> {
		{
			let res: u8 = temp_pool
				.hset(
					format!("users:{}:cmd", player_id),
					[
						("command", cmd.command),
						("cmd_timeout", cmd.timeout.to_string()),
					],
				)
				.await?;
			Ok(res)
		}
	}
	/// Gets a player's requested command, if none returns None
	/// Gets the available areas from the triviador game state
	pub(crate) async fn get_player_cmd(
		temp_pool: &RedisPool,
		player_id: i32,
		game_id: u32,
	) -> Result<Option<Cmd>, anyhow::Error> {
		let res: HashMap<String, String> = temp_pool
			.hgetall(format!("users:{}:cmd", player_id))
			.await?;

		// return if none
		if res.is_empty() {
			return Ok(None);
		}

		let available = AvailableAreas::get_available(temp_pool, game_id).await?;

		Ok(Some(Cmd {
			command: res.get("command").unwrap().to_string(),
			available,
			timeout: res.get("cmd_timeout").unwrap().parse()?,
		}))
	}

	pub(crate) async fn clear_cmd(
		temp_pool: &RedisPool,
		player_id: i32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = temp_pool.del(format!("users:{}:cmd", player_id)).await?;
		Ok(res)
	}
}
