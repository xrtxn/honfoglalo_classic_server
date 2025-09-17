use rand::SeedableRng;
use rand::prelude::{IteratorRandom, StdRng};
use tracing::{trace, warn};

use crate::game_handlers::question_handler::{TipHandler, TipHandlerType};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::users::ServerCommand;

// invisible
// Setup
// 3,1,0
// Announcement,
// 3,1,1
// TipQuestion,
// 3,1,4
// AskDesiredArea,
// 3,1,6
// DesiredAreaResponse,

pub(crate) struct FillRemainingHandler {
	game: SharedTrivGame,
	winner: Option<PlayerName>,
}

impl FillRemainingHandler {
	pub(crate) fn new(game: SharedTrivGame) -> FillRemainingHandler {
		FillRemainingHandler { game, winner: None }
	}
	pub(super) async fn setup(&self) {
		let mut write_game = self.game.write().await;
		write_game.state.game_state = GameState {
			state: 3,
			round: 1,
			phase: 0,
		};
		write_game.state.round_info.mini_phase_num = 0;
	}

	pub(super) async fn announcement(&self) {
		self.game.write().await.state.game_state.phase = 0;
		self.game.send_to_all_active().await;
		trace!("Fill remaining announcement waiting");
		self.game.wait_for_all_active().await;
		trace!("Fill remaining announcement game ready");
	}

	pub(super) async fn tip_question(&mut self) {
		// TODO fix this, not all players should be questioned
		let players = self.game.arc_clone().read().await.utils.clone();
		let mut th = TipHandler::new(self.game.arc_clone(), TipHandlerType::Fill, players).await;
		self.winner = Some(th.handle_all().await);
		self.game
			.write()
			.await
			.state
			.fill_round_winners
			.add_player(self.winner);
	}

	pub(super) async fn ask_desired_area(&self) {
		let active_player = self.winner.unwrap();
		self.game.write().await.state.active_player = Some(active_player);
		let mut game_writer = self.game.write().await;
		game_writer.state.game_state.phase = 4;

		let ri = game_writer.state.round_info.clone();

		game_writer.state.round_info = RoundInfo {
			mini_phase_num: ri.mini_phase_num,
			active_player,
			attacked_player: None,
		};
		drop(game_writer);

		if self
			.game
			.read()
			.await
			.utils
			.get_player(&active_player)
			.unwrap()
			.is_player()
		{
			// let available = AvailableAreas::get_available(temp_pool, self.game_id).await;
			let available = self.game.read().await.state.available_areas.clone();
			Cmd::set_player_cmd(
				self.game.arc_clone(),
				&active_player,
				Some(Cmd::select_command(available, 90)),
			)
			.await;
		}
		self.game.send_to_all_active().await;
		self.game.wait_for_all_active().await;
	}

	pub(super) async fn desired_area_response(&self) {
		self.game.write().await.state.game_state.phase = 6;
		let active_player = self.winner.unwrap();
		self.game.write().await.state.active_player = Some(active_player);
		if self
			.game
			.read()
			.await
			.utils
			.get_player(&active_player)
			.unwrap()
			.is_player()
		{
			match self
				.game
				.recv_command_channel(&active_player)
				.await
				.unwrap()
			{
				ServerCommand::SelectArea(val) => {
					self.new_area_selected(val, active_player).await.unwrap();
				}
				_ => {
					warn!("Invalid command");
				}
			}
		} else {
			let areas = self.game.read().await.state.areas_info.clone();
			let selection = self.game.read().await.state.selection.clone();
			let available_areas =
				AvailableAreas::get_conquerable_areas(&areas, &selection, active_player);

			let mut rng = StdRng::from_entropy();
			let random_area = available_areas.counties().iter().choose(&mut rng).unwrap();
			self.new_area_selected(*random_area as u8, active_player)
				.await
				.unwrap();
		}
		self.game.send_to_all_active().await;
		self.game.wait_for_all_active().await;
		self.game.write().await.state.round_info.mini_phase_num += 1;
	}

	async fn new_area_selected(
		&self,
		selected_area: u8,
		player: PlayerName,
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
			player,
			County::try_from(selected_area).ok(),
		)
		.await?;
		self.game
			.write()
			.await
			.state
			.selection
			.add_selection(player, County::try_from(selected_area)?);

		Ok(())
	}
}
