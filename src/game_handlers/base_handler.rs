use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{trace, warn};

use crate::game_handlers::s_game::SGamePlayer;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::ServerCommand;

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

pub(crate) struct BaseHandler {
	game: SharedTrivGame,
	state: BaseHandlerPhases,
	players: Vec<SGamePlayer>,
}

impl BaseHandler {
	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> BaseHandler {
		BaseHandler {
			game,
			state: BaseHandlerPhases::Announcement,
			players,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_pick(&mut self) {
		self.state = BaseHandlerPhases::StartSelection;
	}

	pub(crate) async fn command(&mut self) {
		let active_player = self.game.read().await.state.active_player.clone().unwrap();
		trace!("active player: {:?}", active_player);
		match self.state {
			BaseHandlerPhases::Announcement => {
				self.base_select_announcement().await.unwrap();
				self.game.send_to_all_players(&self.players).await;
				self.game.wait_for_all_players(&self.players).await;
				trace!("Base select announcement game ready");
				self.game.write().await.state.available_areas = AvailableAreas::all_counties();
			}
			BaseHandlerPhases::StartSelection => {
				self.player_base_select_backend(active_player.rel_id).await;
				let areas = self.game.read().await.state.areas_info.clone();
				let selection = self.game.read().await.state.selection.clone();
				let available =
					AvailableAreas::get_limited_available(&areas, &selection, active_player.rel_id);
				self.game.write().await.state.available_areas = available.clone();
				if active_player.is_player() {
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
			BaseHandlerPhases::SelectionResponse => {
				if !active_player.is_player() {
					let areas = self.game.read().await.state.areas_info.clone();
					let selection = self.game.read().await.state.selection.clone();
					let available = AvailableAreas::get_limited_available(
						&areas,
						&selection,
						active_player.rel_id,
					);
					self.game.write().await.state.available_areas = available.clone();
					let mut rng = StdRng::from_entropy();
					let random_area = available
						.get_counties()
						.into_iter()
						.choose(&mut rng)
						.unwrap();
					self.new_base_selected(*random_area as u8, active_player.rel_id)
						.await;
				} else {
					self.game.write().await.cmd = None;
					let command = self.game.recv_command_channel(&active_player).await;
					match command.unwrap() {
						ServerCommand::SelectArea(val) => {
							self.new_base_selected(val, active_player.rel_id).await;
						}
						_ => {
							warn!("Invalid command");
						}
					}
				}
				self.base_selected_stage().await;
				self.game.send_to_all_players(&self.players).await;
				self.game.wait_for_all_players(&self.players).await;
			}
		}
	}

	pub async fn base_selected_stage(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 1,
			round: 0,
			phase: 3,
		};
	}

	pub async fn new_base_selected(&self, selected_area: u8, rel_id: u8) {
		self.game
			.write()
			.await
			.state
			.available_areas
			.pop_county(&County::try_from(selected_area).unwrap());

		Bases::add_base(
			self.game.arc_clone(),
			PlayerNames::try_from(rel_id).unwrap(),
			Base::new(selected_area),
		)
		.await
		.unwrap();

		Area::base_selected(
			self.game.arc_clone(),
			rel_id,
			County::try_from(selected_area).unwrap(),
		)
		.await
		.unwrap();

		// todo what does this do
		// game.read().unwrap().state.selection = game.read().unwrap().state.base_info.clone();

		TriviadorState::modify_player_score(self.game.arc_clone(), rel_id - 1, 1000)
			.await
			.unwrap();
	}

	async fn base_select_announcement(&self) -> Result<(), anyhow::Error> {
		self.game.write().await.state.game_state = GameState {
			state: 1,
			round: 0,
			phase: 0,
		};
		Ok(())
	}

	async fn player_base_select_backend(&self, game_player_id: u8) {
		self.game.write().await.state.game_state.phase = 1;
		self.game.write().await.state.round_info = RoundInfo {
			mini_phase_num: game_player_id,
			rel_player_id: game_player_id,
			attacked_player: None,
		};
	}
}
