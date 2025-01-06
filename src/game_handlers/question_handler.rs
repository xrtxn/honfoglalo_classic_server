use std::ops::Deref;
use std::sync::Arc;

use fred::clients::RedisPool;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::Mutex;
use tokio::task;
use tracing::{info, warn};

use crate::emulator::Emulator;
use crate::game_handlers::s_game::{get_player_by_rel_id, SGamePlayer};
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::game::{self, SharedTrivGame, TriviadorGame};
use crate::triviador::game_state::GameState;
use crate::triviador::question::{
	Question, QuestionAnswerResult, QuestionStageResponse, TipInfo, TipStageResponse,
};
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::ServerCommand;

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
	game: SharedTrivGame,
	question_handler_type: QuestionHandlerType,
	state: QuestionHandlerPhases,
	players: Vec<SGamePlayer>,
	answer_result: QuestionAnswerResult,
}

impl QuestionHandler {
	pub(crate) async fn handle_all(&mut self) {
		self.command().await;
		self.next();
		self.command().await;
		self.next();
		self.command().await;
		self.next();
		self.command().await;
		self.next();
		self.command().await;
	}

	pub(crate) async fn new(
		game: SharedTrivGame,
		question_handler_type: QuestionHandlerType,
		players: Vec<SGamePlayer>,
	) -> QuestionHandler {
		QuestionHandler {
			game,
			question_handler_type,
			state: QuestionHandlerPhases::SendQuestion,
			players,
			answer_result: QuestionAnswerResult::new(),
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) async fn command(&mut self) {
		match self.state {
			QuestionHandlerPhases::SendQuestion => {
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						self.game.write().await.state.game_state.phase = 4;
					}
					QuestionHandlerType::Battle => {
						self.game.write().await.state.game_state.phase = 4;
					}
				}
				let q = Question::emulate();
				let state = self.game.read().await.state.clone();
				// todo do actual work
				self.game
					.send_xml_channel(
						&self.players[0],
						quick_xml::se::to_string(&QuestionStageResponse::new_question(state, q))
							.unwrap(),
					)
					.await
					.unwrap();
				self.game.wait_for_all_players(&self.players).await;
			}
			QuestionHandlerPhases::GetQuestionResponse => {
				for player in self.players.iter().filter(|x| x.is_player()) {
					match self.game.recv_command_channel(player).await.unwrap() {
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
					let random_answer: u8 = rng.gen_range(1..=4);
					self.answer_result.set_player(player.rel_id, random_answer);
				}
			}
			QuestionHandlerPhases::SendCorrectAnswer => {
				info!("todo but not necessary");
				self.game.write().await.state.game_state.phase += 1;
			}
			QuestionHandlerPhases::SendPlayerAnswers => {
				self.answer_result.good = Some(1);
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						self.game.write().await.state.game_state.phase += 1;
					}
					QuestionHandlerType::Battle => {
						self.game.write().await.state.game_state.phase += 1;
					}
				}
				let state = self.game.read().await.state.clone();
				for player in self.players.iter().filter(|x| x.is_player()) {
					self.game
						.send_xml_channel(
							player,
							quick_xml::se::to_string(&QuestionStageResponse::new_answer_result(
								state.clone(),
								self.answer_result.clone(),
							))
							.unwrap(),
						)
						.await
						.unwrap();
				}
				self.game.wait_for_all_players(&self.players).await;
			}
			QuestionHandlerPhases::SendUpdatedState => {
				match self.question_handler_type {
					QuestionHandlerType::AreaConquer => {
						self.game.write().await.state.game_state.phase += 1;
					}
					QuestionHandlerType::Battle => {
						self.game.write().await.state.game_state.phase = 21;
					}
				}
				let selection = self.game.read().await.state.selection.clone();
				let mut score_increase = Vec::with_capacity(3);
				for player in self.players.clone() {
					if self.answer_result.is_player_correct(player.rel_id) {
						Area::area_occupied(
							self.game.arc_clone(),
							player.rel_id,
							selection.get_player_county(player.rel_id).cloned(),
						)
						.await
						.unwrap();
						score_increase.push(200);
					} else {
						self.game.write().await.state.available_areas.push_county(
							selection.get_player_county(player.rel_id).unwrap().clone(),
						);
						score_increase.push(0);
					}
				}
				TriviadorState::modify_scores(self.game.arc_clone(), score_increase)
					.await
					.unwrap();
				self.game.send_to_all_players(&self.players).await;
				self.game.write().await.state.selection.clear();
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
	game: SharedTrivGame,
	tip_handler_type: TipHandlerType,
	state: TipHandlerPhases,
	players: Vec<SGamePlayer>,
	tip_info: TipInfo,
}

impl TipHandler {
	pub(crate) async fn new(
		game: SharedTrivGame,
		stage_type: TipHandlerType,
		players: Vec<SGamePlayer>,
	) -> TipHandler {
		TipHandler {
			game,
			tip_handler_type: stage_type,
			state: TipHandlerPhases::SendTipRequest,
			players,
			tip_info: TipInfo::new(),
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) async fn handle_all(&mut self) -> SGamePlayer {
		self.send_tip_request().await;
		self.get_tip_response().await;
		self.send_player_answers().await
	}

	async fn send_tip_request(&self) {
		match self.tip_handler_type {
			TipHandlerType::Fill => {
				self.game.write().await.state.game_state.phase = 1;
			}
			TipHandlerType::Battle => {
				self.game.write().await.state.game_state.phase = 10;
			}
		}
		let q = Question::emulate();
		let state = self.game.read().await.state.clone();
		self.game
			.send_xml_channel(
				&self.players[0],
				quick_xml::se::to_string(&TipStageResponse::new_tip(state, q)).unwrap(),
			)
			.await
			.unwrap();
		self.game.wait_for_all_players(&self.players).await;
	}

	async fn get_tip_response(&mut self) {
		let start = std::time::Instant::now();
		let tip_info = Arc::new(Mutex::new(self.tip_info.clone()));

		// Spawn tasks for each player
		let tasks: Vec<_> = self
			.players
			.iter()
			.map(|player| {
				// Clone what you need outside the async move
				let player = player.clone();
				let game = self.game.arc_clone();
				let tip_info = Arc::clone(&tip_info);

				task::spawn(async move {
					if player.is_player() {
						match game.recv_command_channel(&player).await {
							Ok(ServerCommand::TipAnswer(ans)) => {
								tip_info.lock().await.add_player_tip(
									player.rel_id,
									ans,
									start.elapsed().as_secs_f32(),
								);
							}
							_ => {
								todo!();
							}
						}
					} else {
						let mut rng = StdRng::from_entropy();
						let random_answer = rng.gen_range(1..100);
						tip_info.lock().await.add_player_tip(
							player.rel_id,
							random_answer,
							start.elapsed().as_secs_f32(),
						);
					}
				})
			})
			.collect();

		// Wait for all tasks to finish
		futures::future::join_all(tasks).await;

		// Update self.tip_info after collecting all results
		self.tip_info = tip_info.lock().await.clone();
	}
	async fn send_player_answers(&self) -> SGamePlayer {
		// todo placeholder
		let good = 1;
		println!("{:?}", self.tip_info);
		// both need to be increased by 2
		self.game.write().await.state.game_state.phase += 2;

		let state = self.game.read().await.state.clone();
		let tip_stage_response =
			TipStageResponse::new_tip_result(state.clone(), self.tip_info.clone(), good);
		for player in self.players.iter().filter(|x| x.is_player()) {
			self.game
				.send_xml_channel(
					player,
					quick_xml::se::to_string(&tip_stage_response).unwrap(),
				)
				.await
				.unwrap();
		}
		self.game.wait_for_all_players(&self.players).await;
		let winner = tip_stage_response.tip_result.unwrap();
		get_player_by_rel_id(self.players.clone(), winner.winner)
	}
}
