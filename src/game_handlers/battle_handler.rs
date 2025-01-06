use std::time::Duration;

use fred::prelude::{KeysInterface, RedisPool};
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handlers::question_handler::{QuestionHandler, QuestionHandlerType};
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::users::ServerCommand;

#[derive(PartialEq, Clone)]
enum BattleHandlerPhases {
	// invisible
	Setup,
	// 4,1,0
	Announcement,
	// 4,1,1
	AskAttackingArea,
	// 4,1,3
	AttackedAreaResponse,
	// 4,1,4
	Question,
	// 4,1,6
	// send answerresult
	// 4,1,21
	SendUpdatedState,
}

impl BattleHandlerPhases {
	fn new() -> BattleHandlerPhases {
		BattleHandlerPhases::Announcement
	}

	fn next(&mut self) {
		match self {
			BattleHandlerPhases::Setup => *self = BattleHandlerPhases::Announcement,
			BattleHandlerPhases::Announcement => *self = BattleHandlerPhases::AskAttackingArea,
			BattleHandlerPhases::AskAttackingArea => {
				*self = BattleHandlerPhases::AttackedAreaResponse
			}
			BattleHandlerPhases::AttackedAreaResponse => *self = BattleHandlerPhases::Question,
			BattleHandlerPhases::Question => *self = BattleHandlerPhases::SendUpdatedState,
			BattleHandlerPhases::SendUpdatedState => {
				*self = {
					error!("Overstepped the phases, returning to AskDesiredArea");
					BattleHandlerPhases::AskAttackingArea
				}
			}
		}
	}
}

pub(crate) struct BattleHandler {
	game: SharedTrivGame,
	state: BattleHandlerPhases,
	players: Vec<SGamePlayer>,
}

impl BattleHandler {
	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> BattleHandler {
		BattleHandler {
			game,
			state: BattleHandlerPhases::Setup,
			players,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = BattleHandlerPhases::Announcement;
	}

	pub(crate) fn new_player_pick(&mut self) {
		self.state = BattleHandlerPhases::AskAttackingArea;
	}

	pub(crate) async fn command(&mut self, active_player: Option<SGamePlayer>) {
		match self.state {
			BattleHandlerPhases::Setup => {
				self.battle_setup().await;
			}
			BattleHandlerPhases::Announcement => {
				self.game.write().await.state.game_state.phase = 0;
				self.game.send_to_all_players(&self.players).await;
				trace!("Battle announcement waiting");
				self.game.wait_for_all_players(&self.players).await;
				trace!("Battle announcement game ready");
			}
			BattleHandlerPhases::AskAttackingArea => {
				let active_player = active_player.unwrap();
				self.game.write().await.state.active_player = Some(active_player.clone());
				self.ask_area_battle_backend(active_player.rel_id).await;
				if active_player.is_player() {
					let available = self.game.read().await.state.available_areas.clone();
					Cmd::set_player_cmd(
						self.game.arc_clone(),
						&active_player,
						Some(Cmd::select_command(available, 90)),
					)
					.await;
				}
				self.game.send_to_all_players(&self.players).await;
				trace!("Send select cmd waiting");
				self.game.wait_for_all_players(&self.players).await;
				trace!("Send select cmd game ready");
			}
			BattleHandlerPhases::AttackedAreaResponse => {
				let active_player = active_player.unwrap();
				if !active_player.is_player() {
					let available_areas = self.game.read().await.state.available_areas.clone();
					let mut rng = StdRng::from_entropy();
					let random_area = available_areas
						.get_counties()
						.into_iter()
						.choose(&mut rng)
						.unwrap();
					self.new_area_selected(*random_area as u8, active_player.rel_id)
						.await;
				} else {
					// Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();
					Cmd::set_player_cmd(self.game.arc_clone(), &active_player, None).await;

					match self
						.game
						.recv_command_channel(&active_player)
						.await
						.unwrap()
					{
						ServerCommand::SelectArea(val) => {
							self.new_area_selected(val, active_player.rel_id).await;
						}
						_ => {
							warn!("Invalid command");
						}
					}
				}
				self.area_selected_stage().await;

				self.game.send_to_all_players(&self.players).await;
				trace!("Common game ready waiting");
				self.game.wait_for_all_players(&self.players).await;
				trace!("Common game ready");
			}
			BattleHandlerPhases::Question => {
				let mut qh = QuestionHandler::new(
					self.game.arc_clone(),
					QuestionHandlerType::Battle,
					self.players.clone(),
				)
				.await;
				qh.handle_all().await;
			}
			BattleHandlerPhases::SendUpdatedState => {
				self.game.wait_for_all_players(&self.players).await;
			}
		}
	}

	async fn battle_setup(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 4,
			round: 1,
			phase: 0,
		};
		self.game.write().await.state.available_areas = AvailableAreas::all_counties();
	}

	async fn ask_area_battle_backend(&self, game_player_id: u8) {
		// sets phase to 1
		// let mut res = GameState::set_phase(temp_pool, game_id, 1).await?;
		let mut write_game = self.game.write().await;
		write_game.state.game_state.phase = 1;
		write_game.state.round_info.mini_phase_num += 1;
		write_game.state.round_info.rel_player_id = game_player_id;
		write_game.state.round_info.attacked_player = Some(0);
	}

	pub async fn area_selected_stage(&self) {
		// sets phase to 3
		// let res: u8 = GameState::incr_phase(temp_pool, game_id, 2).await?;
		self.game.write().await.state.game_state.phase += 2;
	}

	pub async fn new_area_selected(&self, selected_area: u8, game_player_id: u8) {
		// AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;
		self.game
			.write()
			.await
			.state
			.available_areas
			.pop_county(&County::try_from(selected_area).unwrap());
		self.game.write().await.state.selection.add_selection(
			PlayerNames::try_from(game_player_id).unwrap(),
			County::try_from(selected_area).unwrap(),
		);
	}
}
