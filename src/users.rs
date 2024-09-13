use fred::prelude::*;

pub struct User {}
impl User {
	pub async fn reset(tmppool: &RedisPool, id: i32) -> Result<(), anyhow::Error> {
		// todo reset properly
		let _: String = tmppool.flushall(false).await?;
		tmppool
			.hset::<u8, _, _>(
				format!("users:{}:login_state", id),
				[("is_listen_ready", "false"), ("is_logged_in", "false")],
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
	pub async fn get_next_listen(tmppool: &RedisPool, id: i32) -> Option<String> {
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
		let res: String = tmppool
			.hget(format!("users:{}:login_state", id), "is_listen_ready")
			.await?;
		Ok(res.parse::<bool>()?)
	}
	pub async fn set_listen_ready(
		tmppool: &RedisPool,
		id: i32,
		is_ready: bool,
	) -> Result<bool, anyhow::Error> {
		tmppool
			.hset::<bool, _, _>(
				format!("users:{}:login_state", id),
				("is_listen_ready", is_ready),
			)
			.await
			.map_err(|e| anyhow::anyhow!(e))
	}

	pub async fn get_is_logged_in(tmppool: &RedisPool, id: i32) -> Result<bool, anyhow::Error> {
		let res: String = tmppool
			.hget(format!("users:{}:login_state", id), "is_logged_in")
			.await?;
		Ok(res.parse::<bool>()?)
	}

	pub async fn set_is_logged_in(
		tmppool: &RedisPool,
		id: i32,
		is_logged_in: bool,
	) -> Result<bool, anyhow::Error> {
		let is_logged_in: bool = tmppool
			.hset(
				format!("users:{}:login_state", id),
				("is_logged_in", is_logged_in),
			)
			.await?;
		Ok(is_logged_in)
	}
}
