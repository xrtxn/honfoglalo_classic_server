use fred::prelude::*;
use fred::types::KeyspaceEvent;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tracing::{error, info, trace};

use crate::triviador::county::Cmd;
use crate::triviador::{AvailableAreas, GameState, PlayerInfo, TriviadorGame, TriviadorState};
use crate::users;
use crate::users::{ServerCommand, User};

pub struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(tmppool: &RedisPool, game_id: u32) {
		let game = TriviadorGame::new_game(tmppool, game_id).await.unwrap();
		let player_id = game.players.pd1.id;
		let subscriber = Builder::default_centralized().build().unwrap();
		let ready_subscriber = subscriber.clone_new();
		subscriber.init().await.unwrap();
		ready_subscriber.init().await.unwrap();
		ready_subscriber
			.psubscribe(format!("__keyspace*__:users:{}:is_game_ready", player_id))
			.await
			.unwrap();

		subscriber
			.psubscribe(format!("__keyspace*__:users:{}:is_listen_ready", player_id))
			.await
			.unwrap();

		let mut keyspace_rx = subscriber.keyspace_event_rx();
		while let Ok(_) = keyspace_rx.recv().await {
			if User::get_listen_state(tmppool, player_id).await.unwrap() {
				let gamestate = GameState::get_gamestate(tmppool, game_id).await.unwrap();
				match gamestate.state {
					11 => {
						TriviadorGame::announcement_stage(tmppool, game_id)
							.await
							.unwrap();
						send_player_game(tmppool, game_id, player_id).await;
					}
					1 => match gamestate.phase {
						0 => {
							TriviadorGame::select_bases_stage(tmppool, game_id)
								.await
								.unwrap();

							Cmd::set_player_cmd(
								tmppool,
								player_id,
								Cmd {
									command: "SELECT".to_string(),
									available: Some(AvailableAreas::all_counties()),
									timeout: 100,
								},
							)
							.await
							.unwrap();
							send_player_game(tmppool, game_id, player_id).await;
						}
						1 => {
							// todo modify this to continue even if player gives no response
							wait_for_game_ready(&ready_subscriber, tmppool, player_id).await;
							select! {
								_ = User::subscribe_command(player_id) => {
									trace!("Select area command received");
								}
								_ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
									trace!("Timeout reached");
								}
							}
							Cmd::clear_cmd(tmppool, player_id).await.unwrap();
							TriviadorGame::base_selected_stage(tmppool, game_id)
								.await
								.unwrap();

							match User::get_server_command(tmppool, player_id).await.unwrap() {
								ServerCommand::SelectBase(val) => {
									TriviadorGame::new_base_selected(tmppool, game_id, val, 1)
										.await
										.unwrap();
								}
							}
							send_player_game(tmppool, game_id, player_id).await;
						}
						3 => {}
						_ => {
							todo!()
						}
					},
					_ => {
						todo!()
					}
				}
			} else {
				// wait_for_redis_event(tmppool, &anim_subscriber, player_id).await;
				trace!("Small performance penalty, listen ready is false after checking")
			}
		}
	}
}

async fn wait_for_game_ready(client: &RedisClient, tmppool: &RedisPool, player_id: i32) {
	if User::get_game_ready_state(tmppool, player_id)
		.await
		.unwrap()
	{
		trace!("Player already ready");
		return;
	}
	let mut sub = client.keyspace_event_rx();
	let _res = sub.recv().await.unwrap();
	trace!("received <READY >");
}

async fn send_player_game(tmppool: &RedisPool, game_id: u32, player_id: i32) {
	User::set_game_ready_state(tmppool, player_id, false)
		.await
		.unwrap();
	let mut resp = TriviadorGame::get_triviador(tmppool, game_id)
		.await
		.unwrap();
	resp.cmd = Cmd::get_player_cmd(tmppool, player_id, game_id)
		.await
		.unwrap();
	let asd = resp.clone();
	let xml = quick_xml::se::to_string(&asd).unwrap();
	User::push_listen_queue(tmppool, player_id, xml.as_str())
		.await
		.unwrap();
}
