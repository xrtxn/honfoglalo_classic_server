use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use fred::prelude::*;
use tracing::trace;

pub struct User {}
pub enum ServerCommand {
	SelectBase(u8),
}

impl Display for ServerCommand {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			ServerCommand::SelectBase(base_num) => write!(f, "select_base,{}", base_num),
		}
	}
}
impl FromStr for ServerCommand {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parts: Vec<&str> = s.split(',').collect();
		match parts[0] {
			"select_base" => Ok(ServerCommand::SelectBase(parts[1].parse()?)),
			_ => Err(anyhow::anyhow!("Invalid command")),
		}
	}
}
impl User {
	pub async fn reset(tmppool: &RedisPool, id: i32) -> Result<(), anyhow::Error> {
		// todo reset properly
		let _: String = tmppool.flushall(false).await?;
		let _: bool = tmppool
			.set(
				format!("users:{}:is_logged_in", id),
				false,
				None,
				None,
				false,
			)
			.await?;
		Ok(())
		// tmppool.del::<u8, _>("listen_queue").await.unwrap();
	}

	pub async fn push_listen_queue(
		tmppool: &RedisPool,
		id: i32,
		queue: &str,
	) -> Result<(), anyhow::Error> {
		let _: u8 = tmppool
			.rpush(format!("users:{}:listen_queue", id), queue)
			.await?;
		Ok(())
	}
	pub async fn pop_listen_queue(tmppool: &RedisPool, id: i32) -> Option<String> {
		let res: Option<String> = tmppool
			.lpop(format!("users:{}:listen_queue", id), Some(1))
			.await
			.unwrap_or_else(|_| None);
		res
	}

	pub async fn is_listen_empty(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: u8 = tmppool.exists(format!("users:{}:listen_queue", id)).await?;
		Ok(res == 0)
	}

	pub async fn is_listen_ready(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: String = tmppool.get(format!("users:{}:is_listen_ready", id)).await?;
		Ok(res.parse::<bool>()?)
	}
	pub async fn set_listen_state(
		tmppool: &RedisPool,
		id: i32,
		is_ready: bool,
	) -> Result<(), anyhow::Error> {
		let _: bool = tmppool
			.set(
				format!("users:{}:is_listen_ready", id),
				is_ready,
				None,
				None,
				false,
			)
			.await?;
		Ok(())
	}

	pub async fn get_listen_state(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: bool = tmppool.get(format!("users:{}:is_listen_ready", id)).await?;
		Ok(res)
	}

	pub async fn set_game_ready_state(
		tmppool: &RedisPool,
		id: i32,
		is_ready: bool,
	) -> Result<(), anyhow::Error> {
		let _: bool = tmppool
			.set(
				format!("users:{}:is_game_ready", id),
				is_ready,
				None,
				None,
				false,
			)
			.await?;
		User::set_listen_state(tmppool, id, is_ready).await?;
		Ok(())
	}

	pub async fn get_game_ready_state(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: Option<String> = tmppool.get(format!("users:{}:is_game_ready", id)).await?;
		match res {
			None => {
				trace!("No game ready state found for player {}", id);
				Ok(false)
			}
			Some(val) => val.parse::<bool>().map_err(Into::into),
		}
	}

	pub async fn get_is_logged_in(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: String = tmppool.get(format!("users:{}:is_logged_in", id)).await?;
		Ok(res.parse::<bool>()?)
	}

	pub async fn set_is_logged_in(
		tmppool: &RedisPool,
		id: i32,
		is_logged_in: bool,
	) -> Result<bool, anyhow::Error> {
		let res: bool = tmppool
			.set(
				format!("users:{}:is_logged_in", id),
				is_logged_in,
				None,
				None,
				false,
			)
			.await?;
		Ok(res)
	}

	// this approach may not work for all commands
	pub async fn set_server_command(
		tmppool: &RedisPool,
		id: i32,
		command: ServerCommand,
	) -> Result<(), anyhow::Error> {
		trace!("Setting server command: {}", command);
		let _: String = tmppool
			.set(
				format!("users:{}:server_command", id),
				command.to_string(),
				None,
				None,
				false,
			)
			.await?;
		Ok(())
	}

	pub async fn get_server_command(
		tmppool: &RedisPool,
		id: i32,
	) -> Result<ServerCommand, anyhow::Error> {
		let res: String = tmppool.get(format!("users:{}:server_command", id)).await?;
		trace!("Getting server command: {}", res);
		Ok(res.parse()?)
	}

	pub async fn subscribe_command(player_id: i32) {
		let subscriber = Builder::default_centralized().build().unwrap();
		subscriber.init().await.unwrap();
		subscriber
			.psubscribe(format!("__keyspace*__:users:{}:server_command", player_id))
			.await
			.unwrap();
		let mut sub = subscriber.keyspace_event_rx();
		let _res = sub.recv().await.unwrap();
	}
}
