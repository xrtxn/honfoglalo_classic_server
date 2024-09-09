use fred::prelude::*;

pub struct User {
	id: i32,
	name: String,
}
impl User {
	pub async fn reset(tmppool: &RedisPool, id: i32) {
		// todo reset properly
		let _: String = tmppool.flushall(false).await.unwrap();
		tmppool
			.hset::<u8, _, _>(
				format!("users:{}:login_state", id),
				[("is_listen_ready", "false"), ("is_logged_in", "false")],
			)
			.await
			.unwrap();
		// tmppool.del::<u8, _>("listen_queue").await.unwrap();
	}

	pub async fn push_listen_queue(tmppool: &RedisPool, id: i32, queue: &str) {
		tmppool
			.rpush(format!("users:{}:listen_queue", id), queue)
			.await
			.unwrap()
	}
	pub async fn get_next_listen(tmppool: &RedisPool, id: i32) -> Option<String> {
		let res: Option<String> = tmppool
			.lpop(format!("users:{}:listen_queue", id), Some(1))
			.await
			.unwrap();
		res
	}

	pub async fn is_listen_empty(tmppool: &RedisPool, id: i32) -> bool {
		let res: u8 = tmppool
			.exists(format!("users:{}:listen_queue", id))
			.await
			.unwrap();
		res == 0
	}

	pub async fn is_listen_ready(tmppool: &RedisPool, id: i32) -> bool {
		let res: String = tmppool
			.hget(format!("users:{}:login_state", id), "is_listen_ready")
			.await
			.unwrap();
		res.parse::<bool>().unwrap()
	}
	pub async fn set_listen_ready(tmppool: &RedisPool, id: i32, is_ready: bool) -> bool {
		tmppool
			.hset::<bool, _, _>(
				format!("users:{}:login_state", id),
				("is_listen_ready", is_ready),
			)
			.await
			.unwrap()
	}

	pub async fn get_is_logged_in(tmppool: &RedisPool, id: i32) -> bool {
		let res: String = tmppool
			.hget(format!("users:{}:login_state", id), "is_logged_in")
			.await
			.unwrap();
		res.parse::<bool>().unwrap()
	}

	pub async fn set_is_logged_in(tmppool: &RedisPool, id: i32, is_logged_in: bool) -> bool {
		tmppool
			.hset::<bool, _, _>(
				format!("users:{}:login_state", id),
				("is_logged_in", is_logged_in),
			)
			.await
			.unwrap()
	}
}
