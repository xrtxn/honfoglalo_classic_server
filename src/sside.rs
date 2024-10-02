use fred::prelude::*;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tokio::{join, select};
use tracing::{error, info, trace, warn};

use crate::triviador::county::Cmd;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::{available_area::AvailableAreas, game::TriviadorGame};
use crate::users::{ServerCommand, User};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PlayerType {
	Player,
	Bot,
}

struct SGame {
	game_state: SGameState,
	game_phase: SGamePhase,
	active_player: u8,
	selected_bases_num: u8,
	players: Vec<SGamePlayer>,
}

impl SGame {
	fn new(players: Vec<SGamePlayer>) -> SGame {
		SGame {
			game_state: SGameState::new(),
			game_phase: SGamePhase::new(),
			active_player: 1,
			selected_bases_num: 0,
			players,
		}
	}

	fn next(&mut self) {
		match self.game_state {
			SGameState::BaseSelect => {
				if self.game_phase != SGamePhase::End {
					self.game_phase.next(self.selected_bases_num)
				} else {
					self.game_state = self.game_state.next();
				}
			}
			_ => self.game_state = self.game_state.next(),
		}
	}

	fn next_player(&mut self) {
		self.active_player = match self.active_player {
			1 => 2,
			2 => 3,
			3 => 1,
			_ => {
				warn!("Invalid relative player number, setting to 1");
				1
			}
		}
	}

	async fn action(&mut self, tmppool: &RedisPool, game_id: u32) {
		info!(
			"Player: {:?}",
			self.players.get(self.active_player as usize - 1).unwrap()
		);
		let active_player = self.players.get(self.active_player as usize - 1).unwrap();

		match self.game_state {
			SGameState::Setup => {
				Self::setup_backend(tmppool, game_id).await.unwrap();
				// this must be sent from here as the initial listen state is false
				for player in &self.players {
					if player.is_player() {
						send_player_game(tmppool, game_id, player.id).await;
					}
				}
				trace!("Setup waiting");
				wait_for_game_ready(tmppool, 1).await;
				trace!("Setup game ready");
				return;
			}
			SGameState::BaseSelectAnnouncement => {
				Self::base_select_announcement(tmppool, game_id)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(tmppool, player.id, true).await.unwrap();
				}
				trace!("Base select announcement waiting");
				wait_for_game_ready(tmppool, 1).await;
				trace!("Base select announcement game ready");
			}
			SGameState::BaseSelect => match self.game_phase {
				SGamePhase::SendSelectCmd => {
					SGame::player_base_select_backend(tmppool, game_id, self.active_player)
						.await
						.unwrap();
					if active_player.is_player() {
						Cmd::set_player_cmd(
							tmppool,
							active_player.id,
							Cmd {
								command: "SELECT".to_string(),
								available: Some(AvailableAreas::all_counties()),
								timeout: 90,
							},
						)
						.await
						.unwrap();
					}
					for player in self.players.iter().filter(|x| x.is_player()) {
						User::set_send(tmppool, player.id, true).await.unwrap();
					}
					trace!("Send select cmd waiting");
					wait_for_game_ready(tmppool, 1).await;

					trace!("Send select cmd game ready");
					// count should? start here
				}
				SGamePhase::SelectionResponse => {
					if !active_player.is_player() {
						let available_areas = AvailableAreas::get_available(tmppool, game_id)
							.await
							.unwrap()
							.unwrap();

						let mut rng = StdRng::from_entropy();
						let random_area =
							available_areas.areas.into_iter().choose(&mut rng).unwrap();
						TriviadorGame::new_base_selected(
							tmppool,
							game_id,
							random_area as u8,
							self.active_player,
						)
						.await
						.unwrap();
					} else {
						// trace!("Selection response waiting");
						// trace!("Selection response game ready");
						// todo make this better
						if User::get_server_command(tmppool, active_player.id)
							.await
							.is_ok()
						{
							warn!("Already received server command!!!");
						} else {
							select! {
								_ = {
									trace!("Waiting for select area command");
									User::subscribe_command(active_player.id)
								} => {
									trace!("Select area command received");
								}
								_ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
									trace!("Timeout reached");
								}
							}
						}
						// tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
						Cmd::clear_cmd(tmppool, active_player.id).await.unwrap();

						match User::get_server_command(tmppool, active_player.id)
							.await
							.unwrap()
						{
							ServerCommand::SelectBase(val) => {
								TriviadorGame::new_base_selected(
									tmppool,
									game_id,
									val,
									self.active_player,
								)
								.await
								.unwrap();
							}
						}
						// wait_for_game_ready(tmppool, 1).await;
					}
					TriviadorGame::base_selected_stage(tmppool, game_id)
						.await
						.unwrap();

					for player in self.players.iter().filter(|x| x.is_player()) {
						User::set_send(tmppool, player.id, true).await.unwrap();
					}
					tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
					trace!("Common game ready waiting");
					wait_for_game_ready(tmppool, 1).await;
					trace!("Common game ready");
					self.selected_bases_num += 1;
					self.next_player();
				}
				SGamePhase::End => {
					tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
					warn!("SGamePhase::End reached");
					self.game_state.next();
				}
			},
			SGameState::AreaSelect => todo!("Implement next state"),
		}
		// wait_for_game_ready(tmppool, 1).await;
	}

	async fn setup_backend(tmppool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 11,
				gameround: 0,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}

	async fn base_select_announcement(
		tmppool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}

	async fn player_base_select_backend(
		tmppool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		let mut res: u8 = GameState::set_gamestate(
			tmppool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 1,
			},
		)
		.await?;

		res += RoundInfo::set_roundinfo(
			tmppool,
			game_id,
			RoundInfo {
				last_player: game_player_id,
				next_player: game_player_id,
			},
		)
		.await?;

		AvailableAreas::set_available(tmppool, game_id, AvailableAreas::all_counties()).await?;
		Ok(())
	}
}

#[derive(PartialEq, Clone)]
enum SGamePhase {
	SendSelectCmd,
	SelectionResponse,
	End,
}

impl SGamePhase {
	fn new() -> SGamePhase {
		SGamePhase::SendSelectCmd
	}

	fn next(&mut self, selected_bases_num: u8) {
		match self {
			SGamePhase::SendSelectCmd => *self = SGamePhase::SelectionResponse,
			SGamePhase::SelectionResponse => {
				if selected_bases_num < 3 {
					warn!("Overflown phase");
					*self = SGamePhase::SendSelectCmd
				} else {
					*self = SGamePhase::End
				}
			}
			SGamePhase::End => {}
		}
	}
}

#[derive(Clone)]
enum SGameState {
	Setup,
	BaseSelectAnnouncement,
	BaseSelect,
	AreaSelect,
}

impl SGameState {
	fn new() -> SGameState {
		SGameState::Setup
	}

	fn next(&self) -> SGameState {
		match self {
			SGameState::Setup => SGameState::BaseSelectAnnouncement,
			SGameState::BaseSelectAnnouncement => SGameState::BaseSelect,
			SGameState::BaseSelect => SGameState::AreaSelect,
			SGameState::AreaSelect => todo!("Implement next phase"),
		}
	}
}

#[derive(Eq, Hash, PartialEq, Debug)]
struct SGamePlayer {
	player_type: PlayerType,
	id: i32,
	rel_id: u8,
}

impl SGamePlayer {
	fn new(player_type: PlayerType, id: i32, rel_id: u8) -> SGamePlayer {
		SGamePlayer {
			player_type,
			id,
			rel_id,
		}
	}

	fn is_player(&self) -> bool {
		self.player_type == PlayerType::Player
	}
}

pub struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(tmppool: &RedisPool, game_id: u32) {
		let game = TriviadorGame::new_game(tmppool, game_id).await.unwrap();
		let sgame_players = vec![
			SGamePlayer::new(PlayerType::Player, game.players.pd1.id, 1),
			SGamePlayer::new(PlayerType::Bot, game.players.pd2.id, 2),
			SGamePlayer::new(PlayerType::Bot, game.players.pd3.id, 3),
		];
		let listen_sub = Builder::default_centralized().build().unwrap();
		listen_sub.init().await.unwrap();
		for player in sgame_players.iter().filter(|x| x.is_player()) {
			listen_sub
				.psubscribe(format!("__keyspace*__:users:{}:is_listen_ready", player.id))
				.await
				.unwrap();
		}
		let mut keyspace_rx = listen_sub.keyspace_event_rx();

		let pool_move = tmppool.clone();
		tokio::spawn(async move {
			let tmppool = pool_move;
			while let Ok(event) = keyspace_rx.recv().await {
				let player_id = event
					.key
					.as_str_lossy()
					.split(':')
					.nth(1)
					.unwrap()
					.parse::<i32>()
					.unwrap();
				if User::get_listen_state(&tmppool, player_id).await.unwrap() {
					// todo replace
					while User::get_send(&tmppool, player_id).await.unwrap() != true {
						tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
					}
					send_player_game(&tmppool, game_id, player_id).await;
				} else {
					// trace!("Small performance penalty, listen ready is false after checking")
				}
			}
		});

		// initial setup
		let mut sgame = SGame::new(sgame_players);
		loop {
			sgame.action(tmppool, game_id).await;
			sgame.next();
		}
	}
}

async fn wait_for_game_ready(tmppool: &RedisPool, player_id: i32) {
	// todo improve this, add timeout
	// todo wait for send to become false
	let ready = User::get_game_ready_state(tmppool, player_id).await;
	if ready.unwrap_or_else(|_| false) {
		warn!("FIXME: implement if the user is ready before waiting");
	}
	// todo this is a workaround
	let ready_sub = Builder::default_centralized().build().unwrap();
	ready_sub.init().await.unwrap();
	ready_sub
		.psubscribe(format!("__keyspace*__:users:{}:is_game_ready", player_id))
		.await
		.unwrap();
	let mut sub = ready_sub.keyspace_event_rx();

	while User::get_send(tmppool, player_id).await.unwrap() {
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
		sub.recv().await.unwrap();
	}

	trace!("received <READY >");
}

async fn send_player_game(tmppool: &RedisPool, game_id: u32, player_id: i32) {
	User::set_send(&tmppool, player_id, false).await.unwrap();
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
