use std::sync::Arc;

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, trace, warn};

use super::s_game::GamePlayerInfo;
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
	answer: Option<u8>,
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
		players: GamePlayerInfo,
	) -> QuestionHandler {
		QuestionHandler {
			game: game.arc_clone(),
			question_handler_type,
			question_players: players,
			answer_result: QuestionAnswerResult::new(),
			answer: None,
		}
	}

	pub(crate) async fn answer_result(&self) -> QuestionAnswerResult {
		self.answer_result.clone()
	}

	pub(super) async fn send_question(&mut self) {
		trace!("send_question");
		match self.question_handler_type {
			QuestionHandlerType::AreaConquer => {
				self.game.write().await.state.game_state.phase = 4;
			}
			QuestionHandlerType::Battle => {
				self.game.write().await.state.game_state.phase = 4;
			}
		}
		let db = self.game.arc_clone().write().await.db.clone();
		let q = Some(Question::get_from_db(&db).await);
		self.answer = q.as_ref().unwrap().good;
		let state = self.game.read().await.state.clone();
		let utils = self.game.read().await.utils.clone();
		let iter = utils.active_players_stream();
		futures::stream::StreamExt::for_each_concurrent(iter, None, |player| {
			let game = self.game.arc_clone();
			let state = state.clone();
			let q = q.clone();
			let mut qsr = QuestionStageResponse::new_question(state, q.unwrap());
			if self.question_players.get_player(player).is_none() {
				qsr.cmd = None; // do not send cmd to non question players
			}
			async move {
				if let Err(e) = game
					.send_xml_channel(player, quick_xml::se::to_string(&qsr).unwrap())
					.await
				{
					warn!("Failed to send question to player {}: {}", player, e);
				}
			}
		})
		.await;
	}

	async fn get_question_response(&mut self) {
		trace!("get_question_response");
		let answer_result = Arc::new(Mutex::new(self.answer_result.clone()));
		let iter = self.question_players.players_with_info_stream();
		trace!("get_question_response iter: {:?}", self.question_players);
		futures::stream::StreamExt::for_each_concurrent(iter, None, |(player, info)| {
			let game = self.game.arc_clone();
			let player = *player;
			let answer_result = Arc::clone(&answer_result);
			async move {
				if info.is_player() {
					match game
						.loop_recv_command(&player, ServerCommand::QuestionAnswer(0))
						.await
						.unwrap()
					{
						ServerCommand::QuestionAnswer(ans) => {
							trace!("got_question_response: {:?}", player);
							answer_result.lock().await.set_player_answer(&player, ans);
						}
						_ => {
							error!("Unable to loop wait for player");
						}
					}
				} else {
					tokio::time::sleep(std::time::Duration::from_millis(7331)).await; // Simulate delay for bots
					let mut rng = StdRng::from_entropy();
					let random_answer: u8 = rng.gen_range(1..=4);
					answer_result
						.lock()
						.await
						.set_player_answer(&player, random_answer);
				}
			}
		})
		.await;
		self.answer_result = answer_result.lock().await.clone();
	}

	async fn send_correct_answer(&self) {
		//it is unnecessary to send the correct answer number here
		self.game.write().await.state.game_state.phase += 1;
	}

	async fn send_player_answers(&mut self) {
		trace!("send_player_answers");
		self.answer_result.good = self.answer;
		match self.question_handler_type {
			QuestionHandlerType::AreaConquer => {
				self.game.write().await.state.game_state.phase += 1;
			}
			QuestionHandlerType::Battle => {
				self.game.write().await.state.game_state.phase += 1;
			}
		}
		// todo find a better way than cloning this, possible arc
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
				let mut iter = self.question_players.players_stream();
				while let Some(player) = iter.next().await {
					if self.answer_result.is_player_correct(player) {
						Area::area_occupied(
							self.game.arc_clone(),
							*player,
							selection.get_player_county(player).cloned(),
						)
						.await
						.unwrap();
						self.game
							.write()
							.await
							.state
							.players_points
							.change_player_points(player, 200);
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
	good: Option<i32>,
}

impl TipHandler {
	pub(crate) async fn new(
		game: SharedTrivGame,
		stage_type: TipHandlerType,
		players: GamePlayerInfo,
	) -> TipHandler {
		TipHandler {
			game: game.arc_clone(),
			tip_players: players,
			tip_handler_type: stage_type,
			tip_info: TipInfo::new(),
			good: None,
		}
	}

	pub(crate) async fn handle_all(&mut self) -> PlayerName {
		self.send_tip_request().await;
		self.get_tip_response().await;
		self.send_player_answers().await
	}

	async fn send_tip_request(&mut self) {
		match self.tip_handler_type {
			TipHandlerType::Fill => {
				self.game.write().await.state.game_state.phase = 1;
			}
			TipHandlerType::Battle => {
				self.game.write().await.state.game_state.phase = 10;
			}
		}
		let db = self.game.arc_clone().write().await.db.clone();
		let tq = TipQuestion::get_from_db(&db).await;
		self.good = tq.good;
		let state = self.game.read().await.state.clone();
		let utils = self.game.read().await.utils.clone();
		let iter = utils.active_players_stream();
		futures::stream::StreamExt::for_each_concurrent(iter, None, |player| {
			let game = self.game.arc_clone();
			let mut tip_stage = TipStageResponse::new_tip_question(state.clone(), tq.clone());
			if self.tip_players.get_player(player).is_none() {
				tip_stage.cmd = None;
			}
			async move {
				if let Err(e) = game
					.send_xml_channel(player, quick_xml::se::to_string(&tip_stage).unwrap())
					.await
				{
					warn!("Failed to send tip request to player {}: {}", player, e);
				}
			}
		})
		.await;
	}

	// todo fix elapsed time
	async fn get_tip_response(&mut self) {
		let start = std::time::Instant::now();
		let tip_info = Arc::new(Mutex::new(self.tip_info.clone()));

		let iter = self.tip_players.players_with_info_stream();
		futures::stream::StreamExt::for_each_concurrent(iter, None, |(player, info)| {
			let game = self.game.arc_clone();
			let tip_info = Arc::clone(&tip_info);
			let player = *player;

			async move {
				if info.is_player() {
					match game
						.loop_recv_command(&player, ServerCommand::TipAnswer(0))
						.await
					{
						Ok(ServerCommand::TipAnswer(ans)) => {
							tip_info.lock().await.add_player_tip(
								player,
								ans,
								start.elapsed().as_secs_f32(),
							);
						}
						_ => {
							error!("Unable to loop wait for player");
						}
					}
				} else {
					tokio::time::sleep(std::time::Duration::from_millis(7331)).await; // Simulate delay for bots
					let mut rng = StdRng::from_entropy();
					let random_answer = rng.gen_range(1..100);
					tip_info.lock().await.add_player_tip(
						player,
						random_answer,
						start.elapsed().as_secs_f32(),
					);
				}
			}
		})
		.await;

		// Update self.tip_info after collecting all results
		self.tip_info = tip_info.lock().await.clone();
	}

	//todo cleanup
	async fn send_player_answers(&self) -> PlayerName {
		// todo placeholder
		let good = self.good.unwrap();
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
		// Wait at least 1 second for players to receive the answer
		tokio::join!(
			self.game.wait_for_all_active(),
			tokio::time::sleep(std::time::Duration::from_millis(1000))
		);
		let winner = tip_stage_response.tip_result.unwrap();
		winner.winner
	}
}
