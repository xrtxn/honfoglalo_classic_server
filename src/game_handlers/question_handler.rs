use std::sync::Arc;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{trace, warn};

use super::s_game::GamePlayerInfo;
use crate::emulator::Emulator;
use crate::triviador::areas::Area;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::question::{
	Question, QuestionAnswerResult, QuestionStageResponse, TipInfo, TipQuestion, TipStageResponse,
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
	question_players: GamePlayerInfo,
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
			game: game.arc_clone(),
			question_handler_type,
			question_players: GamePlayerInfo::from(players),
			answer_result: QuestionAnswerResult::new(),
		}
	}

	pub(crate) async fn answer_result(&self) -> QuestionAnswerResult {
		self.answer_result.clone()
	}

	pub(super) async fn send_question(&self) {
		trace!("send_question");
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
		let utils = self.game.read().await.utils.clone();
		let mut iter = utils.active_players_stream();
		while let Some(player) = iter.next().await {
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
		let mut iter = self.question_players.players_with_info_stream();
		while let Some((player, info)) = iter.next().await {
			if info.is_player() {
				match self.game.recv_command_channel(player).await.unwrap() {
					ServerCommand::QuestionAnswer(ans) => {
						trace!("got_question_response: {:?}", player);
						self.answer_result.set_player_answer(player, ans);
					}
					_ => {
						trace!("get_question_response: {:?}", player);
						warn!("Invalid command");
					}
				}
			} else {
				let mut rng = StdRng::from_entropy();
				let random_answer: u8 = rng.gen_range(1..=1);
				// let random_answer: u8 = rng.gen_range(1..=4);
				self.answer_result.set_player_answer(player, random_answer);
			}
		}
	}

	async fn send_correct_answer(&self) {
		//it is unnecessary to send the correct answer number here
		self.game.write().await.state.game_state.phase += 1;
	}

	async fn send_player_answers(&mut self) {
		trace!("send_player_answers");
		self.answer_result.good = Some(1);
		match self.question_handler_type {
			QuestionHandlerType::AreaConquer => {
				self.game.write().await.state.game_state.phase += 1;
			}
			QuestionHandlerType::Battle => {
				self.game.write().await.state.game_state.phase += 1;
			}
		}
		// todo find a better way than cloning this possible arc
		let state = self.game.read().await.state.clone();
		let utils = self.game.read().await.utils.clone();
		let mut iter = utils.active_players_stream();
		while let Some(player) = iter.next().await {
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
				let mut iter = self.question_players.active_players_stream();
				while let Some(player) = iter.next().await {
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
				self.game.send_to_all_active().await;
			}
			QuestionHandlerType::Battle => {
				let mut write_game = self.game.write().await;
				write_game.state.game_state.phase = 21;
				// handle everything else in battle handler
			}
		}
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
	tip_players: GamePlayerInfo,
	tip_handler_type: TipHandlerType,
	tip_info: TipInfo,
}

impl TipHandler {
	pub(crate) async fn new(game: SharedTrivGame, stage_type: TipHandlerType) -> TipHandler {
		let players = game.action_players().await;
		TipHandler {
			game: game.arc_clone(),
			tip_players: GamePlayerInfo::from(players),
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
		let tq = TipQuestion::emulate();
		let state = self.game.read().await.state.clone();
		let utils = self.game.read().await.utils.clone();
		let mut iter = utils.active_players_stream();
		while let Some(player) = iter.next().await {
			let mut tip_stage = TipStageResponse::new_tip_question(state.clone(), tq.clone());
			if self.tip_players.get_player(&player).is_none() {
				tip_stage.cmd = None;
			}

			self.game
				.send_xml_channel(player, quick_xml::se::to_string(&tip_stage).unwrap())
				.await
				.unwrap();
		}
		self.game.wait_for_all_active().await;
	}

	// todo fix elapsed time
	async fn get_tip_response(&mut self) {
		let start = std::time::Instant::now();
		let tip_info = Arc::new(Mutex::new(self.tip_info.clone()));

		let mut join_handles = Vec::new();

		let mut iter = self.tip_players.players_stream();
		while let Some(player) = iter.next().await {
			let game = self.game.arc_clone();
			let is_player = self.tip_players.get_player(&player).unwrap().is_player();
			let tip_info = Arc::clone(&tip_info);
			let player = *player;

			let handle = tokio::spawn(async move {
				if is_player {
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
					tokio::time::sleep(std::time::Duration::from_millis(1234)).await; // Simulate delay for bots
					let mut rng = StdRng::from_entropy();
					let random_answer = rng.gen_range(1..100);
					tip_info.lock().await.add_player_tip(
						player,
						random_answer,
						start.elapsed().as_secs_f32(),
					);
				}
			});
			join_handles.push(handle);
		}

		futures::future::join_all(join_handles).await;

		// Update self.tip_info after collecting all results
		self.tip_info = tip_info.lock().await.clone();
	}

	//todo cleanup
	async fn send_player_answers(&self) -> PlayerName {
		// todo placeholder
		let good = 1;
		// both need to be increased by 2
		self.game.write().await.state.game_state.phase += 2;

		let state = self.game.read().await.state.clone();
		let tip_stage_response =
			TipStageResponse::new_tip_result(state.clone(), self.tip_info.clone(), good);
		let utils = self.game.read().await.utils.clone();
		let mut iter = utils.active_players_stream();
		while let Some(player) = iter.next().await {
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
