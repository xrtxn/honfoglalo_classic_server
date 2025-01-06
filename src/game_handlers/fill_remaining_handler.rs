use std::time::Duration;

use fred::clients::RedisPool;
use fred::prelude::KeysInterface;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handlers::question_handler::{TipHandler, TipHandlerType};
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::ServerCommand;

#[derive(PartialEq, Clone)]
enum FillRemainingHandlerPhases {
	// invisible
	Setup,
	// 3,1,0
	Announcement,
	// 3,1,1
	TipQuestion,
	// 3,1,4
	AskDesiredArea,
	// 3,1,6
	DesiredAreaResponse,
}

impl FillRemainingHandlerPhases {
	fn new() -> FillRemainingHandlerPhases {
		FillRemainingHandlerPhases::Announcement
	}

	fn next(&mut self) {
		match self {
			FillRemainingHandlerPhases::Setup => *self = FillRemainingHandlerPhases::Announcement,
			FillRemainingHandlerPhases::Announcement => {
				*self = FillRemainingHandlerPhases::TipQuestion
			}
			FillRemainingHandlerPhases::TipQuestion => {
				*self = FillRemainingHandlerPhases::AskDesiredArea
			}
			FillRemainingHandlerPhases::AskDesiredArea => {
				*self = FillRemainingHandlerPhases::DesiredAreaResponse
			}
			FillRemainingHandlerPhases::DesiredAreaResponse => {
				*self = {
					error!("Overstepped the phases, returning to AskDesiredArea");
					FillRemainingHandlerPhases::TipQuestion
				}
			}
		}
	}
}

pub(crate) struct FillRemainingHandler {
	game: SharedTrivGame,
	state: FillRemainingHandlerPhases,
	players: Vec<SGamePlayer>,
	winner: Option<SGamePlayer>,
}

impl FillRemainingHandler {
	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> FillRemainingHandler {
		FillRemainingHandler {
			game,
			state: FillRemainingHandlerPhases::Setup,
			players,
			winner: None,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = FillRemainingHandlerPhases::Announcement;
	}

	pub(crate) async fn setup(&self) {
		self.fill_setup().await;
	}

	pub(crate) async fn announcement(&mut self) {
		self.game.write().await.state.game_state.phase = 0;
		self.game.send_to_all_players(&self.players).await;
		trace!("Fill remaining announcement waiting");
		self.game.wait_for_all_players(&self.players).await;
		trace!("Fill remaining announcement game ready");
	}

	pub(crate) async fn tip_question(&mut self) {
		let mut th = TipHandler::new(
			self.game.arc_clone(),
			TipHandlerType::Fill,
			self.players.clone(),
		)
		.await;
		self.winner = Some(th.handle_all().await);
		self.game
			.write()
			.await
			.add_fill_round_winner(self.winner.as_ref().unwrap().rel_id)
			.await;
	}

	pub(crate) async fn ask_desired_area(&mut self) {
		let active_player = self.winner.clone().unwrap();
		self.game.write().await.state.active_player = Some(active_player.clone());
		self.fill_area_select_backend(active_player.rel_id)
			.await
			.unwrap();
		if active_player.is_player() {
			// let available = AvailableAreas::get_available(temp_pool, self.game_id).await;
			let available = self.game.read().await.state.available_areas.clone();
			Cmd::set_player_cmd(
				self.game.arc_clone(),
				&active_player,
				Some(Cmd::select_command(available, 90)),
			)
			.await;
		}
		self.game.send_to_all_players(&self.players).await;
		self.game.wait_for_all_players(&self.players).await;
	}

	pub(crate) async fn desired_area_response(&mut self) {
		self.game.write().await.state.game_state.phase = 6;
		let active_player = self.winner.clone().unwrap();
		self.game.write().await.state.active_player = Some(active_player.clone());
		if active_player.is_player() {
			match self
				.game
				.recv_command_channel(&active_player)
				.await
				.unwrap()
			{
				ServerCommand::SelectArea(val) => {
					self.new_area_selected(val, active_player.rel_id)
						.await
						.unwrap();
				}
				_ => {
					warn!("Invalid command");
				}
			}
		} else {
			let areas = self.game.read().await.state.areas_info.clone();
			let selection = self.game.read().await.state.selection.clone();
			let available_areas =
				AvailableAreas::get_limited_available(&areas, &selection, active_player.rel_id);

			let mut rng = StdRng::from_entropy();
			let random_area = available_areas
				.get_counties()
				.into_iter()
				.choose(&mut rng)
				.unwrap();
			self.new_area_selected(*random_area as u8, active_player.rel_id)
				.await
				.unwrap();
		}
		self.game.send_to_all_players(&self.players).await;
		self.game.wait_for_all_players(&self.players).await;
		self.game.write().await.state.round_info.mini_phase_num += 1;
	}

	async fn fill_setup(&self) {
		let mut write_game = self.game.write().await;
		write_game.state.game_state = GameState {
			state: 3,
			round: 1,
			phase: 0,
		};
		write_game.state.round_info.mini_phase_num = 0;
	}

	async fn fill_area_select_backend(&self, winner_rel_id: u8) -> Result<(), anyhow::Error> {
		let mut game = self.game.write().await;
		game.state.game_state.phase = 4;

		let ri = game.state.round_info.clone();

		game.state.round_info = RoundInfo {
			mini_phase_num: ri.mini_phase_num,
			rel_player_id: winner_rel_id,
			attacked_player: None,
		};

		Ok(())
	}

	pub async fn area_selected_stage(&self) {
		// sets phase to 3
		self.game.write().await.state.game_state.phase += 2;
	}

	pub async fn new_area_selected(
		&self,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		// AvailableAreas::pop_county(self.game.arc_clone(),
		// County::try_from(selected_area)?).await;
		self.game
			.write()
			.await
			.state
			.available_areas
			.pop_county(&County::try_from(selected_area)?);
		Area::area_occupied(
			self.game.arc_clone(),
			game_player_id,
			County::try_from(selected_area).ok(),
		)
		.await?;
		self.game.write().await.state.selection.add_selection(
			PlayerNames::try_from(game_player_id)?,
			County::try_from(selected_area)?,
		);

		Ok(())
	}
}
