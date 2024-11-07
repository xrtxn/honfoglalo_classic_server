use fred::clients::RedisPool;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info, warn};

use crate::emulator::Emulator;
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::game_state::GameState;
use crate::triviador::question::{
	Question, QuestionAnswerResult, QuestionStageResponse, TipStageResponse,
};
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::{ServerCommand, User};

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

	pub(crate) async fn new(players: Vec<SGamePlayer>, game_id: u32) -> QuestionHandler {
		QuestionHandler {
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
				GameState::set_phase(temp_pool, self.game_id, 4)
					.await
					.unwrap();
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
					let random_answer: u8 = rng.gen_range(1..4);
					self.answer_result.set_player(player.rel_id, random_answer);
				}
			}
			QuestionHandlerPhases::SendCorrectAnswer => {
				info!("todo but not necessary");
				GameState::incr_phase(temp_pool, self.game_id, 1)
					.await
					.unwrap();
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

enum TipHandlerPhases {
	// 4,1,10
	SendTipRequest,
	GetTipResponse,
	// 4,1,12
	SendPlayerAnswers,
	// 4,1,21
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

pub(crate) struct TipRequestHandler {
	state: TipHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
	answer_result: QuestionAnswerResult,
}

impl TipRequestHandler {
	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool) {
		match self.state {
			TipHandlerPhases::SendTipRequest => {
				GameState::set_phase(temp_pool, self.game_id, 10)
					.await
					.unwrap();
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
			TipHandlerPhases::GetTipResponse => {
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
			TipHandlerPhases::SendPlayerAnswers => {}
			TipHandlerPhases::SendUpdatedState => {}
		}
	}

	pub(crate) async fn handle_all(&mut self, temp_pool: &RedisPool) {
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
		self.next();
		self.command(temp_pool).await;
	}

	pub(crate) async fn new(players: Vec<SGamePlayer>, game_id: u32) -> TipRequestHandler {
		TipRequestHandler {
			state: TipHandlerPhases::SendTipRequest,
			players,
			game_id,
			answer_result: QuestionAnswerResult::new(),
		}
	}
}
