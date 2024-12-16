use std::fmt::{Display, Formatter};
use std::str::FromStr;

use fred::prelude::*;
use tracing::log::warn;
use tracing::trace;

pub struct User {}
#[derive(Clone, Debug)]
pub enum ServerCommand {
	SelectArea(u8),
	QuestionAnswer(u8),
	TipAnswer(i32),
}

impl Display for ServerCommand {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			ServerCommand::SelectArea(area_num) => write!(f, "select_area,{}", area_num),
			ServerCommand::QuestionAnswer(answer_num) => {
				write!(f, "answer,{}", answer_num)
			}
			ServerCommand::TipAnswer(tip) => {
				write!(f, "tip,{}", tip)
			}
		}
	}
}

impl FromStr for ServerCommand {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parts: Vec<&str> = s.split(',').collect();
		match parts[0] {
			"select_area" => Ok(ServerCommand::SelectArea(parts[1].parse()?)),
			"answer" => Ok(ServerCommand::QuestionAnswer(parts[1].parse()?)),
			"tip" => Ok(ServerCommand::TipAnswer(parts[1].parse()?)),
			_ => Err(anyhow::anyhow!("Invalid command")),
		}
	}
}
impl User {
	pub async fn push_listen_queue(
		temp_pool: &RedisPool,
		id: i32,
		queue: &str,
	) -> Result<(), anyhow::Error> {
		let _: u8 = temp_pool
			.rpush(format!("users:{}:listen_queue", id), queue)
			.await?;
		Ok(())
	}

	pub async fn is_listen_ready(temp_pool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: String = temp_pool
			.get(format!("users:{}:is_listen_ready", id))
			.await?;
		Ok(res.parse::<bool>()?)
	}

	pub async fn clear_server_command(temp_pool: &RedisPool, id: i32) -> Result<(), anyhow::Error> {
		let _: String = temp_pool
			.del(format!("users:{}:server_command", id))
			.await?;
		Ok(())
	}

	pub async fn get_server_command(
		temp_pool: &RedisPool,
		id: i32,
	) -> Result<ServerCommand, anyhow::Error> {
		let res: String = temp_pool
			.get(format!("users:{}:server_command", id))
			.await?;
		trace!("Getting server command: {}", res);
		User::clear_server_command(temp_pool, id).await?;
		Ok(res.parse()?)
	}

	pub async fn subscribe_server_command(player_id: i32) {
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
