use std::time::Duration;

use fred::clients::RedisPool;
use fred::prelude::*;
use tokio::select;
use tracing::{trace, warn};

use crate::triviador::cmd::Cmd;
use crate::triviador::game::TriviadorGame;
use crate::users::User;

pub(crate) mod area_conquer_handler;
pub(crate) mod base_handler;
pub(crate) mod battle_handler;
mod fill_remaining_handler;
pub(crate) mod question_handler;
pub(crate) mod s_game;
pub(crate) mod server_game_handler;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PlayerType {
	Player,
	Bot,
}

pub async fn wait_for_game_ready(temp_pool: &RedisPool, player_id: i32) {
	// todo improve this, add timeout
	// todo check if player is already ready?
	let ready_sub = Builder::default_centralized().build().unwrap();
	ready_sub.init().await.unwrap();
	ready_sub
		.psubscribe(format!("__keyspace*__:users:{}:is_listen_ready", player_id))
		.await
		.unwrap();
	let mut sub = ready_sub.keyspace_event_rx();
	let mut is_ready = false;
	while !is_ready {
		sub.recv().await.unwrap();
		if !User::is_listen_ready(&temp_pool, player_id).await.unwrap() {
			trace!("User is not ready");
			continue;
		}
		is_ready = true;
	}
}

pub(crate) async fn send_player_commongame(temp_pool: &RedisPool, game_id: u32, player_id: i32) {
	let mut resp = TriviadorGame::get_triviador(temp_pool, game_id)
		.await
		.unwrap();
	resp.cmd = Cmd::get_player_cmd(temp_pool, player_id, game_id)
		.await
		.unwrap();
	let xml = quick_xml::se::to_string(&resp.clone()).unwrap();
	User::push_listen_queue(temp_pool, player_id, xml.as_str())
		.await
		.unwrap();
}

async fn send_player_string(temp_pool: &RedisPool, player_id: i32, response: String) {
	let xml = quick_xml::se::to_string(&response).unwrap();
	User::push_listen_queue(temp_pool, player_id, xml.as_str())
		.await
		.unwrap();
}

pub(crate) async fn player_timeout_timer(
	temp_pool: &RedisPool,
	active_player_id: i32,
	timeout: Duration,
) -> bool {
	if User::get_server_command(temp_pool, active_player_id)
		.await
		.is_ok()
	{
		warn!("Already received server command!!!");
		true
	} else {
		select! {
			_ = {
				trace!("Waiting for server command for player {}", active_player_id);
				User::subscribe_server_command(active_player_id)
			} => {
				trace!("Server command received for player {}", active_player_id);
				true
			}
			_ = tokio::time::sleep(timeout) => {
				warn!("Timeout reached");
				false
			}
		}
	}
}
