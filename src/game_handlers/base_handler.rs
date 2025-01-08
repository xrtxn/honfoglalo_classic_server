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
use crate::users::ServerCommand;

// Announcement,
// StartSelection,
// SelectionResponse,

pub(crate) struct BaseHandler {
	game: SharedTrivGame,
	players: Vec<SGamePlayer>,
}

impl BaseHandler {
	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> BaseHandler {
		BaseHandler { game, players }
	}

	pub(super) async fn new_base_selected(&self, selected_area: u8, rel_id: u8) {
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

		self.game
			.write()
			.await
			.state
			.players_points
			.set_player_points(&rel_id, 1000);
	}

	pub(super) async fn announcement(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 1,
			round: 0,
			phase: 0,
		};
		self.game.send_to_all_players(&self.players).await;
		self.game.wait_for_all_players(&self.players).await;
		trace!("Base select announcement game ready");
		self.game.write().await.state.available_areas = AvailableAreas::all_counties();
	}

	pub(super) async fn start_selection(&self) {
		let mut game_writer = self.game.write().await;
		let active_player = game_writer.state.active_player.clone().unwrap();
		game_writer.state.game_state.phase = 1;
		game_writer.state.round_info = RoundInfo {
			mini_phase_num: active_player.rel_id,
			rel_player_id: active_player.rel_id,
			attacked_player: None,
		};
		let game_reader = game_writer.downgrade();
		let areas = &game_reader.state.areas_info;
		let available = AvailableAreas::get_base_areas(areas, active_player.rel_id);
		drop(game_reader);
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

	pub(super) async fn selection_response(&self) {
		let active_player = self.game.read().await.state.active_player.clone().unwrap();
		if !active_player.is_player() {
			let areas = self.game.read().await.state.areas_info.clone();
			let available = AvailableAreas::get_base_areas(&areas, active_player.rel_id);
			self.game.write().await.state.available_areas = available.clone();
			let mut rng = StdRng::from_entropy();
			let random_area = available.counties().iter().choose(&mut rng).unwrap();
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
		self.game.write().await.state.game_state = GameState {
			state: 1,
			round: 0,
			phase: 3,
		};
		self.game.send_to_all_players(&self.players).await;
		self.game.wait_for_all_players(&self.players).await;
	}
}
