use fred::prelude::*;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tokio::{join, select};
use tracing::{error, info, trace, warn};

use crate::triviador::areas::Area;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::triviador::{available_area, available_area::AvailableAreas, game::TriviadorGame};
use crate::users::{ServerCommand, User};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PlayerType {
	Player,
	Bot,
}

struct SGame {
	game_state: SGameState,
	base_handler: BaseHandler,
	area_handler: AreaHandler,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl SGame {
	fn new(players: Vec<SGamePlayer>, game_id: u32) -> SGame {
		SGame {
			game_state: SGameState::new(),
			base_handler: BaseHandler::new(players.clone(), game_id),
			area_handler: AreaHandler::new(players.clone(), game_id),
			players,
			game_id,
		}
	}

	fn next(&mut self) {
		self.game_state = self.game_state.next()
	}

	async fn command(&mut self, temp_pool: &RedisPool) {
		match self.game_state {
			SGameState::Setup => {
				Self::setup_backend(temp_pool, self.game_id).await.unwrap();
				// this must be sent from here as the initial listen state is false
				for player in &self.players {
					if player.is_player() {
						send_player_game(temp_pool, self.game_id, player.id).await;
					}
				}
				trace!("Setup waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Setup game ready");
			}
			SGameState::BaseSelection => {
				// announcement for players
				for player in self.players.iter().filter(|x| x.is_player()) {
					self.base_handler.command(temp_pool, player.clone()).await;
				}

				// pick a base for everyone
				for player in &self.players {
					self.base_handler.new_pick();
					self.base_handler.command(temp_pool, player.clone()).await;
					self.base_handler.next();
					self.base_handler.command(temp_pool, player.clone()).await;
				}
				Selection::clear(temp_pool, self.game_id).await.unwrap()
			}
			SGameState::AreaSelection => {
				// announcement for players
				for player in self.players.iter().filter(|x| x.is_player()) {
					self.area_handler.command(temp_pool, player.clone()).await;
				}

				loop {
					// select an area for everyone
					for player in &self.players {
						self.area_handler.new_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
						self.area_handler.next();
						self.area_handler.command(temp_pool, player.clone()).await;
					}
					todo!();
				}
			}
			SGameState::Battle => {
				todo!("Implement next phase")
			}
		}
	}

	async fn setup_backend(temp_pool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
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
}

#[derive(Clone)]
enum SGameState {
	Setup,
	BaseSelection,
	AreaSelection,
	Battle,
}

impl SGameState {
	fn new() -> SGameState {
		SGameState::Setup
	}

	fn next(&self) -> SGameState {
		match self {
			SGameState::Setup => SGameState::BaseSelection,
			SGameState::BaseSelection => SGameState::AreaSelection,
			SGameState::AreaSelection => SGameState::Battle,
			SGameState::Battle => {
				todo!("Implement next phase")
			}
		}
	}
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
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

#[derive(PartialEq, Clone)]
enum BaseHandlerPhases {
	Announcement,
	StartSelection,
	SelectionResponse,
}

impl BaseHandlerPhases {
	fn new() -> BaseHandlerPhases {
		BaseHandlerPhases::StartSelection
	}

	fn next(&mut self) {
		*self = match self {
			BaseHandlerPhases::Announcement => BaseHandlerPhases::StartSelection,
			BaseHandlerPhases::StartSelection => BaseHandlerPhases::SelectionResponse,
			BaseHandlerPhases::SelectionResponse => BaseHandlerPhases::Announcement,
		}
	}
}

struct BaseHandler {
	state: BaseHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl BaseHandler {
	fn new(players: Vec<SGamePlayer>, game_id: u32) -> BaseHandler {
		BaseHandler {
			state: BaseHandlerPhases::Announcement,
			players,
			game_id,
		}
	}

	fn next(&mut self) {
		self.state.next();
	}

	fn new_pick(&mut self) {
		self.state = BaseHandlerPhases::StartSelection;
	}

	async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {
		match self.state {
			BaseHandlerPhases::Announcement => {
				Self::base_select_announcement(temp_pool, self.game_id)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				trace!("Base select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Base select announcement game ready");
				AvailableAreas::set_available(
					temp_pool,
					self.game_id,
					AvailableAreas::all_counties(),
				)
				.await
				.unwrap();
			}
			BaseHandlerPhases::StartSelection => {
				Self::player_base_select_backend(temp_pool, self.game_id, active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					Cmd::set_player_cmd(
						temp_pool,
						active_player.id,
						Cmd {
							command: "SELECT".to_string(),
							available: AvailableAreas::get_available(temp_pool, self.game_id)
								.await
								.unwrap(),
							timeout: 90,
						},
					)
					.await
					.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				trace!("Send select cmd waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Send select cmd game ready");
			}
			BaseHandlerPhases::SelectionResponse => {
				if !active_player.is_player() {
					let available_areas = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap()
						.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					BaseHandler::new_base_selected(
						temp_pool,
						self.game_id,
						random_area as u8,
						active_player.rel_id,
					)
					.await
					.unwrap();
				} else {
					// todo make this better
					if User::get_server_command(temp_pool, active_player.id)
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
					Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();

					match User::get_server_command(temp_pool, active_player.id)
						.await
						.unwrap()
					{
						ServerCommand::SelectArea(val) => {
							BaseHandler::new_base_selected(
								temp_pool,
								self.game_id,
								val,
								active_player.rel_id,
							)
							.await
							.unwrap();
						}
					}
					User::clear_server_command(temp_pool, active_player.id)
						.await
						.unwrap();
				}
				BaseHandler::base_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
		}
	}

	pub async fn base_selected_stage(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 3,
			},
		)
		.await?;
		Ok(res)
	}

	pub async fn new_base_selected(
		temp_pool: &RedisPool,
		game_id: u32,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;

		Bases::add_base(
			temp_pool,
			game_id,
			PlayerNames::try_from(game_player_id)?,
			Base::new(selected_area),
		)
		.await?;

		Area::base_selected(
			temp_pool,
			game_id,
			game_player_id,
			County::try_from(selected_area)?,
		)
		.await?;

		let res = TriviadorState::set_field(
			temp_pool,
			game_id,
			"selection",
			&Bases::serialize_full(&Bases::get_redis(temp_pool, game_id).await?)?,
		)
		.await?;
		let scores = TriviadorState::get_field(temp_pool, game_id, "players_points").await?;
		let mut scores: Vec<u16> = scores
			.split(',')
			.map(|x| x.parse::<u16>().unwrap())
			.collect();
		scores[game_player_id as usize - 1] += 1000;
		TriviadorState::set_field(
			temp_pool,
			game_id,
			"players_points",
			&format!("{},{},{}", scores[0], scores[1], scores[2]),
		)
		.await?;
		Ok(res)
	}

	async fn base_select_announcement(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
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
		temp_pool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		let mut res: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 1,
				gameround: 0,
				phase: 1,
			},
		)
		.await?;

		res += RoundInfo::set_roundinfo(
			temp_pool,
			game_id,
			RoundInfo {
				last_player: game_player_id,
				next_player: game_player_id,
			},
		)
		.await?;

		Ok(())
	}
}

#[derive(PartialEq, Clone)]
enum AreaHandlerPhases {
	// 2,1,0
	Announcement,
	// 2,1,1
	AskDesiredArea,
	// 2,1,3
	DesiredAreaResponse,
	// 2,1,4
	ShowQuestion,
}

impl AreaHandlerPhases {
	fn new() -> AreaHandlerPhases {
		AreaHandlerPhases::AskDesiredArea
	}

	fn next(&mut self) {
		match self {
			AreaHandlerPhases::Announcement => *self = AreaHandlerPhases::AskDesiredArea,
			AreaHandlerPhases::AskDesiredArea => *self = AreaHandlerPhases::DesiredAreaResponse,
			AreaHandlerPhases::DesiredAreaResponse => *self = AreaHandlerPhases::ShowQuestion,
			AreaHandlerPhases::ShowQuestion => todo!("Implement next phase"),
		}
	}
}

struct AreaHandler {
	state: AreaHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl AreaHandler {
	fn new(players: Vec<SGamePlayer>, game_id: u32) -> AreaHandler {
		AreaHandler {
			state: AreaHandlerPhases::Announcement,
			players,
			game_id,
		}
	}

	fn next(&mut self) {
		self.state.next();
	}

	fn new_pick(&mut self) {
		self.state = AreaHandlerPhases::AskDesiredArea;
	}

	async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {
		match self.state {
			AreaHandlerPhases::Announcement => {
				Self::area_select_announcement(temp_pool, self.game_id)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				trace!("Base select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Base select announcement game ready")
			}
			AreaHandlerPhases::AskDesiredArea => {
				Self::player_area_select_backend(temp_pool, self.game_id, active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					Cmd::set_player_cmd(
						temp_pool,
						active_player.id,
						// todo put this into cmd.rs
						Cmd {
							command: "SELECT".to_string(),
							available: AvailableAreas::get_available(temp_pool, self.game_id)
								.await
								.unwrap(),
							timeout: 90,
						},
					)
					.await
					.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				trace!("Send select cmd waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Send select cmd game ready");
			}
			AreaHandlerPhases::DesiredAreaResponse => {
				if !active_player.is_player() {
					let available_areas = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap()
						.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					AreaHandler::new_area_selected(
						temp_pool,
						self.game_id,
						random_area as u8,
						active_player.rel_id,
					)
					.await
					.unwrap();
				} else {
					// todo make this better
					if User::get_server_command(temp_pool, active_player.id)
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
					Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();

					match User::get_server_command(temp_pool, active_player.id)
						.await
						.unwrap()
					{
						ServerCommand::SelectArea(val) => {
							AreaHandler::new_area_selected(
								temp_pool,
								self.game_id,
								val,
								active_player.rel_id,
							)
							.await
							.unwrap();
						}
					}
					User::clear_server_command(temp_pool, active_player.id)
						.await
						.unwrap();
				}
				AreaHandler::area_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					User::set_send(temp_pool, player.id, true).await.unwrap();
				}
				tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
			AreaHandlerPhases::ShowQuestion => {
				todo!("Implement next phase");
			}
		}
	}

	async fn area_select_announcement(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 2,
				gameround: 1,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}

	async fn player_area_select_backend(
		temp_pool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		let mut res: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 2,
				gameround: 1,
				phase: 1,
			},
		)
		.await?;

		res += RoundInfo::set_roundinfo(
			temp_pool,
			game_id,
			RoundInfo {
				last_player: game_player_id,
				next_player: game_player_id,
			},
		)
		.await?;
		Ok(())
	}

	pub async fn area_selected_stage(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 2,
				gameround: 1,
				phase: 3,
			},
		)
		.await?;
		Ok(res)
	}

	pub async fn new_area_selected(
		temp_pool: &RedisPool,
		game_id: u32,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;

		let mut prev = Selection::get_redis(temp_pool, game_id).await?;
		prev.add_selection(
			PlayerNames::try_from(game_player_id)?,
			County::try_from(selected_area)?,
		);
		let res = Selection::set_redis(temp_pool, game_id, prev).await?;

		Ok(res)
	}
}

pub struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(temp_pool: &RedisPool, game_id: u32) {
		let game = TriviadorGame::new_game(temp_pool, game_id).await.unwrap();
		let server_game_players = vec![
			SGamePlayer::new(PlayerType::Player, game.players.pd1.id, 1),
			SGamePlayer::new(PlayerType::Bot, game.players.pd2.id, 2),
			SGamePlayer::new(PlayerType::Bot, game.players.pd3.id, 3),
		];
		let listen_sub = Builder::default_centralized().build().unwrap();
		listen_sub.init().await.unwrap();
		for player in server_game_players.iter().filter(|x| x.is_player()) {
			listen_sub
				.psubscribe(format!("__keyspace*__:users:{}:is_listen_ready", player.id))
				.await
				.unwrap();
		}
		let mut keyspace_rx = listen_sub.keyspace_event_rx();

		let pool_move = temp_pool.clone();
		tokio::spawn(async move {
			let temp_pool = pool_move;
			while let Ok(event) = keyspace_rx.recv().await {
				let player_id = event
					.key
					.as_str_lossy()
					.split(':')
					.nth(1)
					.unwrap()
					.parse::<i32>()
					.unwrap();
				if User::get_listen_state(&temp_pool, player_id).await.unwrap() {
					// todo replace
					while User::get_send(&temp_pool, player_id).await.unwrap() != true {
						tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
					}
					send_player_game(&temp_pool, game_id, player_id).await;
				} else {
					// trace!("Small performance penalty, listen ready is false after checking")
				}
			}
		});

		// initial setup
		let mut server_game = SGame::new(server_game_players, game_id);
		loop {
			server_game.command(temp_pool).await;
			server_game.next();
		}
	}
}

async fn wait_for_game_ready(temp_pool: &RedisPool, player_id: i32) {
	// todo improve this, add timeout
	// todo wait for send to become false
	let ready = User::get_game_ready_state(temp_pool, player_id).await;
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

	while User::get_send(temp_pool, player_id).await.unwrap() {
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
		sub.recv().await.unwrap();
	}

	trace!("received <READY >");
}

async fn send_player_game(temp_pool: &RedisPool, game_id: u32, player_id: i32) {
	User::set_send(&temp_pool, player_id, false).await.unwrap();
	User::set_game_ready_state(temp_pool, player_id, false)
		.await
		.unwrap();
	let mut resp = TriviadorGame::get_triviador(temp_pool, game_id)
		.await
		.unwrap();
	resp.cmd = Cmd::get_player_cmd(temp_pool, player_id, game_id)
		.await
		.unwrap();
	let asd = resp.clone();
	let xml = quick_xml::se::to_string(&asd).unwrap();
	User::push_listen_queue(temp_pool, player_id, xml.as_str())
		.await
		.unwrap();
}
