use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace};

use crate::game_handlers::question_handler::{QuestionHandler, QuestionHandlerType};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::users::ServerCommand;

// invisible
// Setup,
// 2,1,0
// Announcement,
// 2,1,1
// AskDesiredArea,
// 2,1,3
// DesiredAreaResponse,
// 2,1,4
// Question,
// 2,1,7
// SendUpdatedState,

pub(crate) struct AreaConquerHandler {
	game: SharedTrivGame,
}

impl AreaConquerHandler {
	pub(crate) fn new(game: SharedTrivGame) -> AreaConquerHandler {
		AreaConquerHandler { game }
	}

	pub(super) async fn setup(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 2,
			round: 1,
			phase: 0,
		};
	}

	pub(super) async fn announcement(&self) {
		self.game.write().await.state.game_state.phase = 0;
		self.game.send_to_all_active().await;
		trace!("Area select announcement waiting");
		if self.game.read().await.state.game_state.round == 1 {
			self.game.wait_for_all_active().await;
		}
		trace!("Area select announcement game ready");
	}

	pub(super) async fn ask_desired_area(&self) {
		let game_reader = self.game.read().await;
		let active_player = game_reader.state.active_player.clone().unwrap();
		let areas = &game_reader.state.areas_info;
		let selection = &game_reader.state.selection;
		let available = AvailableAreas::get_conquerable_areas(areas, selection, active_player);
		drop(game_reader);
		self.game.write().await.state.available_areas = available.clone();
		let mut game_writer = self.game.write().await;
		game_writer.state.game_state.phase = 1;

		let num = if game_writer.state.round_info.mini_phase_num == 3 {
			1
		} else {
			game_writer.state.round_info.mini_phase_num + 1
		};

		game_writer.state.round_info = RoundInfo {
			mini_phase_num: num,
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
			.is_player()
		{
			Cmd::set_player_cmd(
				self.game.arc_clone(),
				&active_player,
				Some(Cmd::select_command(available, 90)),
			)
			.await;
		}
		self.game.send_to_all_active().await;
		trace!("Send select cmd waiting");
		self.game.wait_for_all_active().await;
		trace!("Send select cmd game ready");
	}

	pub(super) async fn desired_area_response(&self) {
		let active_player = self.game.read().await.state.active_player.clone().unwrap();
		if self
			.game
			.read()
			.await
			.utils
			.get_player(&active_player)
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
					error!("Invalid command");
				}
			}
			trace!("command received");
		} else {
			let readgame = self.game.read().await;
			let areas = readgame.state.areas_info.clone();
			let selection = readgame.state.selection.clone();
			drop(readgame);
			let available_areas =
				AvailableAreas::get_conquerable_areas(&areas, &selection, active_player);

			let mut rng = StdRng::from_entropy();
			let random_area = available_areas.counties().iter().choose(&mut rng).unwrap();
			self.new_area_selected(*random_area as u8, active_player)
				.await
				.unwrap();
		}
		self.game.write().await.state.game_state.phase = 3;

		self.game.send_to_all_active().await;
		trace!("Common game ready waiting");
		self.game.wait_for_all_active().await;
		trace!("Common game ready");
	}

	pub(super) async fn question(&self) {
		let mut qh =
			QuestionHandler::new(self.game.arc_clone(), QuestionHandlerType::AreaConquer).await;
		qh.handle_all().await;
	}

	pub(super) async fn send_updated_state(&self) {
		// it actually gets sent in the question handler
		self.game.wait_for_all_active().await;
		self.game.write().await.state.round_info.mini_phase_num = 0;
	}

	async fn new_area_selected(
		&self,
		selected_area: u8,
		player: PlayerName,
	) -> Result<(), anyhow::Error> {
		self.game
			.write()
			.await
			.state
			.available_areas
			.pop_county(&County::try_from(selected_area)?);

		self.game
			.write()
			.await
			.state
			.selection
			.add_selection(player, County::try_from(selected_area)?);
		Ok(())
	}
}
