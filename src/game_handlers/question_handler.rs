use fred::clients::RedisPool;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::task;
use tracing::{info, warn};

use crate::emulator::Emulator;
use crate::game_handlers::s_game::{get_player_by_rel_id, SGamePlayer};
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::game_state::GameState;
use crate::triviador::question::{
	Question, QuestionAnswerResult, QuestionStageResponse, TipInfo, TipStageResponse,
};
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::{ServerCommand, User};

pub(crate) enum QuestionHandlerType {
	AreaConquer,
	Battle,
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

pub(crate) struct QuestionHandler {
	question_handler_type: QuestionHandlerType,
	state: QuestionHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
	answer_result: QuestionAnswerResult,
}

impl QuestionHandler {
	pub(crate) async fn handle_all(&mut self, temp_pool: &RedisPool) {
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
	}

	pub(crate) async fn new(
		question_handler_type: QuestionHandlerType,
		players: Vec<SGamePlayer>,
		game_id: u32,
	) -> QuestionHandler {
		QuestionHandler {
			question_handler_type,
			state: QuestionHandlerPhases::SendQuestion,
			players,
			game_id,
			answer_result: QuestionAnswerResult::new(),
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool) {
		match self.state {
			QuestionHandlerPhases::SendQuestion => {
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						GameState::set_phase(temp_pool, self.game_id, 4)
							.await
							.unwrap();
					}
					QuestionHandlerType::Battle => {
						todo!()
					}
				}
				let q = Question::emulate();
				let state = TriviadorState::get_triviador_state(temp_pool, self.game_id)
					.await
					.unwrap();
				User::push_listen_queue(
					temp_pool,
					self.players[0].id,
					quick_xml::se::to_string(&QuestionStageResponse::new_question(state, q))
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
					let random_answer: u8 = rng.gen_range(1..5);
					self.answer_result.set_player(player.rel_id, random_answer);
				}
			}
			QuestionHandlerPhases::SendCorrectAnswer => {
				info!("todo but not necessary");
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						GameState::incr_phase(temp_pool, self.game_id, 1)
							.await
							.unwrap();
					}
					QuestionHandlerType::Battle => {
						todo!()
					}
				}
			}
			QuestionHandlerPhases::SendPlayerAnswers => {
				self.answer_result.good = Some(1);
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						GameState::incr_phase(temp_pool, self.game_id, 1)
							.await
							.unwrap();
					}
					QuestionHandlerType::Battle => {
						todo!()
					}
				}
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
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						GameState::incr_phase(temp_pool, self.game_id, 1)
							.await
							.unwrap();
					}
					QuestionHandlerType::Battle => {
						todo!()
					}
				}
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

pub(crate) enum TipHandlerType {
	Fill,
	Battle,
}

enum TipHandlerPhases {
	// Fill: 3,1,1
	// Battle: 4,1,10
	SendTipRequest,
	GetTipResponse,
	// Fill: 3,1,3
	// Battle: 4,1,12
	SendPlayerAnswers,
	// Fill: 3,1,6
	// Battle: 4,1,21
	SendUpdatedState,
}

impl TipHandlerPhases {
	fn new() -> TipHandlerPhases {
		TipHandlerPhases::SendTipRequest
	}

	fn next(&mut self) {
		match self {
			TipHandlerPhases::SendTipRequest => *self = TipHandlerPhases::GetTipResponse,
			TipHandlerPhases::GetTipResponse => *self = TipHandlerPhases::SendPlayerAnswers,
			TipHandlerPhases::SendPlayerAnswers => *self = TipHandlerPhases::SendUpdatedState,
			TipHandlerPhases::SendUpdatedState => {
				warn!("Overstepped the phases, returning to SendTipRequest");
				*self = TipHandlerPhases::SendTipRequest
			}
		}
	}
}

pub(crate) struct TipHandler {
	tip_handler_type: TipHandlerType,
	state: TipHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
	tip_info: TipInfo,
}

impl TipHandler {
	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) async fn handle_all(&mut self, temp_pool: &RedisPool) -> SGamePlayer {
		self.send_tip_request(temp_pool).await;
		self.get_tip_response(temp_pool).await;
		self.send_player_answers(temp_pool).await
	}

	async fn send_tip_request(&self, temp_pool: &RedisPool) {
		match self.tip_handler_type {
			TipHandlerType::Fill => {
				GameState::set_phase(temp_pool, self.game_id, 1)
					.await
					.unwrap();
			}
			TipHandlerType::Battle => {
				GameState::set_phase(temp_pool, self.game_id, 10)
					.await
					.unwrap();
			}
		}
		let q = Question::emulate();
		let state = TriviadorState::get_triviador_state(temp_pool, self.game_id)
			.await
			.unwrap();
		User::push_listen_queue(
			temp_pool,
			self.players[0].id,
			quick_xml::se::to_string(&TipStageResponse::new_tip(state, q))
				.unwrap()
				.as_str(),
		)
		.await
		.unwrap();
		wait_for_game_ready(temp_pool, 1).await;
	}

	async fn get_tip_response(&mut self, temp_pool: &RedisPool) {
		let start = tokio::time::Instant::now();
		let mut tip_info = TipInfo::new();

		let mut tasks = vec![];
		for player in self.players.clone() {
			let temp_pool = temp_pool.clone();
			let task = task::spawn(async move {
				return if player.is_player() {
					// todo handle no response
					User::subscribe_server_command(player.id).await;
					match User::get_server_command(&temp_pool, player.id)
						.await
						.unwrap()
					{
						ServerCommand::TipAnswer(ans) => {
							// tip_info.add_player_tip(
							(player.rel_id, ans, start.elapsed().as_secs_f32())
							// );
						}
						_ => {
							warn!("Invalid command");
							// todo placeholder
							(player.rel_id, 1, start.elapsed().as_secs_f32())
						}
					}
				} else {
					let mut rng = StdRng::from_entropy();
					let random_answer: i32 = rng.gen_range(1..100);
					// tip_info.add_player_tip(
					(player.rel_id, random_answer, start.elapsed().as_secs_f32())
					// );
				};
			});
			tasks.push(task);
		}
		let res = futures::future::join_all(tasks).await;
		for (rel_id, tip, time) in res.iter().map(|x| x.as_ref().unwrap()) {
			tip_info.add_player_tip(*rel_id, *tip, *time);
		}
		self.tip_info = tip_info;
	}

	async fn send_player_answers(&self, temp_pool: &RedisPool) -> SGamePlayer {
		// todo placeholder
		let good = 1;
		println!("{:?}", self.tip_info);
		// both need to be increased by 2
		GameState::incr_phase(temp_pool, self.game_id, 2)
			.await
			.unwrap();
		let state = TriviadorState::get_triviador_state(temp_pool, self.game_id)
			.await
			.unwrap();
		let tip_stage_response =
			TipStageResponse::new_tip_result(state.clone(), self.tip_info.clone(), good);
		for player in self.players.iter().filter(|x| x.is_player()) {
			User::push_listen_queue(
				temp_pool,
				player.id,
				quick_xml::se::to_string(&tip_stage_response)
					.unwrap()
					.as_str(),
			)
			.await
			.unwrap();
		}
		wait_for_game_ready(temp_pool, 1).await;
		let winner = tip_stage_response.tip_result.unwrap();
		get_player_by_rel_id(self.players.clone(), winner.winner)
	}

	pub(crate) async fn new(
		stage_type: TipHandlerType,
		players: Vec<SGamePlayer>,
		game_id: u32,
	) -> TipHandler {
		TipHandler {
			tip_handler_type: stage_type,
			state: TipHandlerPhases::SendTipRequest,
			players,
			game_id,
			tip_info: TipInfo::new(),
		}
	}
}
