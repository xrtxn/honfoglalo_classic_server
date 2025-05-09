use std::sync::Arc;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::StreamExt;
use tracing::{info, warn};

use super::s_game::GamePlayerInfo;
use crate::emulator::Emulator;
use crate::triviador::areas::Area;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::question::{
	Question, QuestionAnswerResult, QuestionStageResponse, TipInfo, TipStageResponse,
};
use crate::users::ServerCommand;

// 2,1,4
// SendQuestion,
//
// GetQuestionResponse,
//
// 2,1,5 (not sent)
// Sends the correct answer number - this is completely unnecessary
// SendCorrectAnswer,
//
// 2,1,6
// SendPlayerAnswers,
//
// 2,1,7
// SendUpdatedState,

pub(crate) enum QuestionHandlerType {
	AreaConquer,
	Battle,
}

pub(crate) struct QuestionHandler {
	game: SharedTrivGame,
	question_handler_type: QuestionHandlerType,
	action_players: GamePlayerInfo,
	answer_result: QuestionAnswerResult,
}

impl QuestionHandler {
	pub(crate) async fn handle_all(&mut self) {
		self.send_question().await;
		self.get_question_response().await;
		self.send_correct_answer().await;
		self.send_player_answers().await;
		self.send_updated_state().await;
	}

	pub(crate) async fn new(
		game: SharedTrivGame,
		question_handler_type: QuestionHandlerType,
	) -> QuestionHandler {
		let players = game.action_players().await;
		QuestionHandler {
			game,
			question_handler_type,
			action_players: GamePlayerInfo::from(players),
			answer_result: QuestionAnswerResult::new(),
		}
	}

	pub(super) async fn send_question(&self) {
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
		while let Some((player, _)) = self.action_players.active_iter().next().await {
			self.game
				.send_xml_channel(
					&player,
					quick_xml::se::to_string(&QuestionStageResponse::new_question(
						state.clone(),
						q.clone(),
					))
					.unwrap(),
				)
				.await
				.unwrap();
		}
		self.game.wait_for_all_active().await;
	}

	async fn get_question_response(&mut self) {
		while let Some((player, _)) = self.action_players.active_iter().next().await {
			match self.game.recv_command_channel(player).await.unwrap() {
				ServerCommand::QuestionAnswer(ans) => {
					self.answer_result.set_player(player, ans);
				}
				_ => {
					warn!("Invalid command");
				}
			}
		}
		while let Some((player, _)) = self.action_players.inactive_iter().next().await {
			let mut rng = StdRng::from_entropy();
			let random_answer: u8 = rng.gen_range(1..=4);
			self.answer_result.set_player(player, random_answer);
		}
	}

	async fn send_correct_answer(&self) {
		info!("todo but not necessary");
		self.game.write().await.state.game_state.phase += 1;
	}

	async fn send_player_answers(&mut self) {
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
		while let Some((player, _)) = self.action_players.active_iter().next().await {
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
		self.game.wait_for_all_active().await;
	}

	async fn send_updated_state(&self) {
		let selection = self.game.read().await.state.selection.clone();
		match self.question_handler_type {
			QuestionHandlerType::AreaConquer => {
				self.game.write().await.state.game_state.phase += 1;
				for (player, _) in self.action_players.0.iter() {
					if self.answer_result.is_player_correct(&player) {
						Area::area_occupied(
							self.game.arc_clone(),
							*player,
							selection.get_player_county(&player).cloned(),
						)
						.await
						.unwrap();
						self.game
							.write()
							.await
							.state
							.players_points
							.change_player_points(&player, 200);
					} else {
						self.game
							.write()
							.await
							.state
							.available_areas
							.push_county(*selection.get_player_county(player).unwrap());
					}
				}
			}
			QuestionHandlerType::Battle => {
				let mut write_game = self.game.write().await;
				write_game.state.game_state.phase = 21;
				for (player, _) in self.action_players.0.iter() {
					if self.answer_result.is_player_correct(&player) {
						write_game
							.state
							.players_points
							.change_player_points(&player, 200);
					} else {
						write_game
							.state
							.players_points
							.change_player_points(&player, -200);
					}
				}
			}
		}

		self.game.send_to_all_active().await;
	}
}

// Fill: 3,1,1
// Battle: 4,1,10
// SendTipRequest,
// GetTipResponse,
// Fill: 3,1,3
// Battle: 4,1,12
// SendPlayerAnswers,
// Fill: 3,1,6
// Battle: 4,1,21
// SendUpdatedState,

pub(crate) enum TipHandlerType {
	Fill,
	Battle,
}

pub(crate) struct TipHandler {
	game: SharedTrivGame,
	action_players: GamePlayerInfo,
	tip_handler_type: TipHandlerType,
	tip_info: TipInfo,
}

impl TipHandler {
	pub(crate) async fn new(game: SharedTrivGame, stage_type: TipHandlerType) -> TipHandler {
		let players = game.action_players().await;
		TipHandler {
			game,
			action_players: GamePlayerInfo::from(players),
			tip_handler_type: stage_type,
			tip_info: TipInfo::new(),
		}
	}

	pub(crate) async fn handle_all(&mut self) -> PlayerName {
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
		while let Some((player, _)) = self.action_players.active_iter().next().await {
			self.game
				.send_xml_channel(
					player,
					quick_xml::se::to_string(&TipStageResponse::new_tip(state.clone(), q.clone()))
						.unwrap(),
				)
				.await
				.unwrap();
		}
		self.game.wait_for_all_active().await;
	}

	// todo review this
	async fn get_tip_response(&mut self) {
		let start = std::time::Instant::now();
		let tip_info = Arc::new(Mutex::new(self.tip_info.clone()));

		// Spawn tasks for each player
		let tasks: Vec<_> = self
			.action_players
			.0
			.iter()
			.map(|(player, info)| {
				// Clone what you need outside the async move
				let player = player.clone();
				let info = info.clone();
				let game = self.game.arc_clone();
				let tip_info = Arc::clone(&tip_info);

				task::spawn(async move {
					if info.is_player() {
						match game.recv_command_channel(&player).await {
							Ok(ServerCommand::TipAnswer(ans)) => {
								tip_info.lock().await.add_player_tip(
									player,
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
							player,
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
	async fn send_player_answers(&self) -> PlayerName {
		// todo placeholder
		let good = 1;
		// both need to be increased by 2
		self.game.write().await.state.game_state.phase += 2;

		let state = self.game.read().await.state.clone();
		let tip_stage_response =
			TipStageResponse::new_tip_result(state.clone(), self.tip_info.clone(), good);
		while let Some((player, _)) = self.action_players.active_iter().next().await {
			self.game
				.send_xml_channel(
					player,
					quick_xml::se::to_string(&tip_stage_response).unwrap(),
				)
				.await
				.unwrap();
		}
		self.game.wait_for_all_active().await;
		let winner = tip_stage_response.tip_result.unwrap();
		winner.winner
	}
}
