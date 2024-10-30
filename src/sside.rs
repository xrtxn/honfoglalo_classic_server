use std::time::Duration;

use fred::prelude::*;
use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};
use tokio::select;
use tracing::{error, trace, warn};

use crate::emulator::Emulator;
use crate::triviador::areas::Area;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::{GamePlayerData, PlayerNames};
use crate::triviador::game_state::GameState;
use crate::triviador::player_info::PlayerInfo;
use crate::triviador::question::{AnswerResult, Question, QuestionStageResponse};
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::triviador::{available_area::AvailableAreas, game::TriviadorGame};
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
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
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
				loop {
					// announcement for all players
					for player in self.players.iter().filter(|x| x.is_player()) {
						self.area_handler.new_round_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
					}
					// select an area for everyone
					for player in self.players.iter() {
						self.area_handler.new_player_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
						self.area_handler.next();
						self.area_handler.command(temp_pool, player.clone()).await;
					}
					self.area_handler.next();
					self.area_handler
						.command(temp_pool, self.players[0].clone())
						.await;
					GameState::incr_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();
					RoundInfo::set_roundinfo(
						temp_pool,
						self.game_id,
						RoundInfo {
							last_player: 1,
							next_player: 1,
						},
					)
					.await
					.unwrap();
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
				round: 0,
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
					send_player_commongame(temp_pool, self.game_id, player.id).await;
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
					let available = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap();
					Cmd::set_player_cmd(
						temp_pool,
						active_player.id,
						Cmd::select_command(available, 90),
					)
					.await
					.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
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
					player_timeout_timer(temp_pool, active_player.id, Duration::from_secs(60))
						.await;
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
						_ => {
							warn!("Invalid command");
						}
					}
				}
				BaseHandler::base_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
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
				round: 0,
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
		rel_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;

		Bases::add_base(
			temp_pool,
			game_id,
			PlayerNames::try_from(rel_id)?,
			Base::new(selected_area),
		)
		.await?;

		Area::base_selected(temp_pool, game_id, rel_id, County::try_from(selected_area)?).await?;

		let res = TriviadorState::set_field(
			temp_pool,
			game_id,
			"selection",
			&Bases::serialize_full(&Bases::get_redis(temp_pool, game_id).await?)?,
		)
		.await?;
		TriviadorState::modify_player_score(temp_pool, game_id, rel_id - 1, 1000).await?;
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
				round: 0,
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
		let mut res: u8 = GameState::set_phase(temp_pool, game_id, 1).await?;

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
	Question,
	// 2,1,7
	SendUpdatedState,
}

impl AreaHandlerPhases {
	fn new() -> AreaHandlerPhases {
		AreaHandlerPhases::AskDesiredArea
	}

	fn next(&mut self) {
		match self {
			AreaHandlerPhases::Announcement => *self = AreaHandlerPhases::AskDesiredArea,
			AreaHandlerPhases::AskDesiredArea => *self = AreaHandlerPhases::DesiredAreaResponse,
			AreaHandlerPhases::DesiredAreaResponse => *self = AreaHandlerPhases::Question,
			AreaHandlerPhases::Question => *self = AreaHandlerPhases::SendUpdatedState,
			AreaHandlerPhases::SendUpdatedState => {
				*self = {
					error!("Overstepped the phases, returning to AskDesiredArea");
					AreaHandlerPhases::AskDesiredArea
				}
			}
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

	fn new_round_pick(&mut self) {
		self.state = AreaHandlerPhases::Announcement;
	}

	fn new_player_pick(&mut self) {
		self.state = AreaHandlerPhases::AskDesiredArea;
	}

	async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {
		match self.state {
			AreaHandlerPhases::Announcement => {
				let game_state = GameState::get_gamestate(temp_pool, self.game_id)
					.await
					.unwrap();
				if game_state.round == 0 {
					Self::area_select_announcement(temp_pool, self.game_id)
						.await
						.unwrap();
				} else {
					GameState::set_phase(temp_pool, self.game_id, 0)
						.await
						.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Area select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Area select announcement game ready");
			}
			AreaHandlerPhases::AskDesiredArea => {
				Self::player_area_select_backend(temp_pool, self.game_id, active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					let available = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap();
					Cmd::set_player_cmd(
						temp_pool,
						active_player.id,
						Cmd::select_command(available, 90),
					)
					.await
					.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
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
					player_timeout_timer(temp_pool, active_player.id, Duration::from_secs(60))
						.await;
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
						_ => {
							warn!("Invalid command");
						}
					}
				}
				AreaHandler::area_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
			AreaHandlerPhases::Question => {
				let mut qh = QuestionHandler::new(self.players.clone(), self.game_id).await;
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
			}
			AreaHandlerPhases::SendUpdatedState => {
				trace!("Sendupdatedstate waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Sendupdatedstate game ready");
			}
		}
	}

	async fn area_select_announcement(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		// let _ = join!(
		// 	GameState::set_state(temp_pool, game_id, 2),
		// 	GameState::set_phase(temp_pool, game_id, 0),
		// );
		// if GameState::get_gamestate(temp_pool, game_id).await?.round == 0 {
		// 	GameState::set_round(temp_pool, game_id, 1).await?;
		// }
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 2,
				round: 1,
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
		// sets phase to 1
		let mut res = GameState::set_phase(temp_pool, game_id, 1).await?;

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
		// sets phase to 3
		let res: u8 = GameState::incr_phase(temp_pool, game_id, 2).await?;
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

#[derive(PartialEq, Clone)]
enum QuestionHandlerPhases {
	// 2,1,4
	SendQuestion,
	GetQuestionResponse,
	// 2,1,5 (not sent)
	// Sends the correct answer number - this is completely unnecessary
	SendCorrectAnswer,
	// 2,1,6
	SendPlayerAnswers,
	// 2,1,7
	SendUpdatedState,
}

impl QuestionHandlerPhases {
	fn new() -> QuestionHandlerPhases {
		QuestionHandlerPhases::SendQuestion
	}

	fn next(&mut self) {
		match self {
			QuestionHandlerPhases::SendQuestion => {
				*self = QuestionHandlerPhases::GetQuestionResponse
			}
			QuestionHandlerPhases::GetQuestionResponse => {
				*self = QuestionHandlerPhases::SendCorrectAnswer
			}
			QuestionHandlerPhases::SendCorrectAnswer => {
				*self = QuestionHandlerPhases::SendPlayerAnswers
			}
			QuestionHandlerPhases::SendPlayerAnswers => {
				*self = QuestionHandlerPhases::SendUpdatedState
			}
			QuestionHandlerPhases::SendUpdatedState => {
				warn!("Overstepped the phases, returning to SendQuestion");
				*self = QuestionHandlerPhases::SendQuestion
			}
		}
	}
}

struct QuestionHandler {
	state: QuestionHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
	answer_result: AnswerResult,
}

impl QuestionHandler {
	async fn new(players: Vec<SGamePlayer>, game_id: u32) -> QuestionHandler {
		QuestionHandler {
			state: QuestionHandlerPhases::SendQuestion,
			players,
			game_id,
			answer_result: AnswerResult::new(),
		}
	}

	fn next(&mut self) {
		self.state.next();
	}

	async fn command(&mut self, temp_pool: &RedisPool) {
		match self.state {
			QuestionHandlerPhases::SendQuestion => {
				GameState::set_phase(temp_pool, self.game_id, 4)
					.await
					.unwrap();
				let q = Question::emulate();
				let old_state = TriviadorState::get_triviador_state(temp_pool, self.game_id)
					.await
					.unwrap();
				User::push_listen_queue(
					temp_pool,
					self.players[0].id,
					quick_xml::se::to_string(&QuestionStageResponse::new_question(old_state, q))
						.unwrap()
						.as_str(),
				)
				.await
				.unwrap();
				wait_for_game_ready(temp_pool, 1).await;
			}
			QuestionHandlerPhases::GetQuestionResponse => {
				for player in self.players.iter().filter(|x| x.is_player()) {
					// todo handle no response
					User::subscribe_server_command(player.id).await;
					match User::get_server_command(temp_pool, player.id)
						.await
						.unwrap()
					{
						ServerCommand::QuestionAnswer(ans) => {
							self.answer_result.set_player(player.rel_id, ans);
						}
						_ => {
							warn!("Invalid command");
						}
					}
				}
				for player in self.players.iter().filter(|x| !x.is_player()) {
					let mut rng = StdRng::from_entropy();
					let random_answer: u8 = rng.gen_range(1..4);
					self.answer_result.set_player(player.rel_id, random_answer);
				}
			}
			QuestionHandlerPhases::SendCorrectAnswer => {
				warn!("todo but not necessary");
				GameState::incr_phase(temp_pool, self.game_id, 1)
					.await
					.unwrap();
				// todo get all players
				// let self_answer = SelfAnswer::new(self.answer_result.player1.unwrap());
				// for player in self.players.iter().filter(|x| x.is_player()) {
				// 	User::push_listen_queue(
				// 		temp_pool,
				// 		player.id,
				// 		quick_xml::se::to_string(&self_answer).unwrap().as_str(),
				// 	)
				// 	.await
				// 	.unwrap();
				// 	User::set_send(temp_pool, player.id, true).await.unwrap();
				// }
			}
			QuestionHandlerPhases::SendPlayerAnswers => {
				self.answer_result.good = Some(1);
				GameState::incr_phase(temp_pool, self.game_id, 1)
					.await
					.unwrap();
				let state = TriviadorState::get_triviador_state(temp_pool, self.game_id)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					User::push_listen_queue(
						temp_pool,
						player.id,
						quick_xml::se::to_string(&QuestionStageResponse::new_answer_result(
							state.clone(),
							self.answer_result.clone(),
						))
						.unwrap()
						.as_str(),
					)
					.await
					.unwrap();
				}
				wait_for_game_ready(temp_pool, 1).await;
			}
			QuestionHandlerPhases::SendUpdatedState => {
				GameState::incr_phase(temp_pool, self.game_id, 1)
					.await
					.unwrap();
				let selection = Selection::get_redis(temp_pool, self.game_id).await.unwrap();
				let mut score_increase = Vec::with_capacity(3);
				for player in self.players.clone() {
					if self.answer_result.is_player_correct(player.rel_id) {
						Area::area_occupied(
							temp_pool,
							self.game_id,
							player.rel_id,
							selection.get_player_county(player.rel_id).cloned(),
						)
						.await
						.unwrap();
						score_increase.push(200);
					} else {
						AvailableAreas::push_county(
							temp_pool,
							self.game_id,
							selection.get_player_county(player.rel_id).cloned().unwrap(),
						)
						.await
						.unwrap();
						score_increase.push(0);
					}
				}
				TriviadorState::modify_scores(temp_pool, self.game_id, score_increase)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				Selection::clear(temp_pool, self.game_id).await.unwrap();
			}
		}
	}
}

pub struct ServerGameHandler {}

impl ServerGameHandler {
	pub async fn new_friendly(temp_pool: &RedisPool, game_id: u32) {
		let players = PlayerInfo {
			p1_name: "xrtxn".to_string(),
			p2_name: "null".to_string(),
			p3_name: "null".to_string(),
			pd1: GamePlayerData::emu_player(),
			pd2: GamePlayerData::new_bot(),
			pd3: GamePlayerData::new_bot(),
			you: "1,2,3".to_string(),
			game_id,
			room: "1".to_string(),
			rules: "0,0".to_string(),
		};

		let game = TriviadorGame::new_game(temp_pool, game_id, players)
			.await
			.unwrap();
		let server_game_players = vec![
			SGamePlayer::new(PlayerType::Player, game.players.pd1.id, 1),
			SGamePlayer::new(PlayerType::Bot, game.players.pd2.id, 2),
			SGamePlayer::new(PlayerType::Bot, game.players.pd3.id, 3),
		];

		// initial setup
		let mut server_game = SGame::new(server_game_players, game_id);
		loop {
			server_game.command(temp_pool).await;
			server_game.next();
		}
	}
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

async fn send_player_commongame(temp_pool: &RedisPool, game_id: u32, player_id: i32) {
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

async fn player_timeout_timer(
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
