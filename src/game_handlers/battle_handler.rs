use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{trace, warn};

use super::question_handler::{TipHandler, TipHandlerType};
use crate::game_handlers::question_handler::{QuestionHandler, QuestionHandlerType};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::game_state::GameState;
use crate::users::ServerCommand;

// Phases
//
// Setup,
// 4,1,0
// Announcement,
// 4,1,1
// AskAttackingArea,
// 4,1,3
// AttackedAreaResponse,
// 4,1,4
// Question,
// 4,1,6
// sends answerresult
// 4,1,10..
// OptionalTipQuestion,
// 4,1,21
// SendUpdatedState,

pub(crate) struct BattleHandler {
	game: SharedTrivGame,
	// todo
	active_players: Vec<PlayerName>,
}

impl BattleHandler {
	pub(crate) fn new(game: SharedTrivGame) -> BattleHandler {
		BattleHandler {
			game,
			active_players: Vec::with_capacity(2),
		}
	}

	pub(super) async fn setup(&self) {
		let mut write_game = self.game.write().await;
		write_game.state.game_state = GameState {
			state: 4,
			round: 1,
			phase: 0,
		};
		write_game.state.available_areas = AvailableAreas::all_counties();
	}

	pub(super) async fn announcement(&self) {
		self.game.write().await.state.game_state.phase = 0;
		self.game.write().await.state.round_info.mini_phase_num += 1;
		self.game.send_to_all_active().await;
		trace!("Battle announcement waiting");
		self.game.wait_for_all_active().await;
		trace!("Battle announcement game ready");
	}

	pub(super) async fn ask_attacking_area(&self) {
		let mut write_game = self.game.write().await;
		let active_player = write_game.state.active_player.clone().unwrap();
		write_game.state.game_state.phase = 1;
		write_game.state.round_info.active_player = active_player;
		write_game.state.round_info.attacked_player = Some(PlayerName::Nobody);

		let read_game = write_game.downgrade();
		let areas = &read_game.state.areas_info;
		let active_player = self.game.read().await.state.active_player.clone().unwrap();
		let available = AvailableAreas::get_attackable_areas(areas, active_player);
		drop(read_game);
		self.game.write().await.state.available_areas = available.clone();
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

	pub(super) async fn attacked_area_response(&mut self) {
		let active_player = self.game.read().await.state.active_player.clone().unwrap();
		if self
			.game
			.read()
			.await
			.utils
			.get_player(&active_player)
			.is_player()
		{
			Cmd::set_player_cmd(self.game.arc_clone(), &active_player, None).await;

			match self
				.game
				.recv_command_channel(&active_player)
				.await
				.unwrap()
			{
				ServerCommand::SelectArea(val) => {
					self.new_area_selected(val, active_player).await;
					let readgame = self.game.read().await;
					let areas_info = readgame.state.areas_info.clone();
					let attacked_rel_id = areas_info.get_area(&County::try_from(val).unwrap());
					drop(readgame);
					self.game.write().await.state.round_info.attacked_player =
						attacked_rel_id.map(|x| PlayerName::from(x.owner));

					// todo this is bad, only for testing
					let attacked_rel_id = self
						.game
						.read()
						.await
						.state
						.round_info
						.attacked_player
						.unwrap();
					let attacked_player = PlayerName::from(attacked_rel_id);
					//

					self.active_players.push(attacked_player);
					self.active_players.push(active_player);
				}
				_ => {
					warn!("Invalid command");
				}
			}
		} else {
			let available_areas = self.game.read().await.state.available_areas.clone();
			let mut rng = StdRng::from_entropy();
			let random_area = available_areas.counties().iter().choose(&mut rng).unwrap();

			let readgame = self.game.read().await;
			let attacked_rel_id = readgame.state.areas_info.get_area(random_area);
			self.game.write().await.state.round_info.attacked_player =
				attacked_rel_id.map(|x| x.owner);

			self.new_area_selected(*random_area as u8, active_player)
				.await;
		}
		self.game.write().await.state.game_state.phase = 3;

		self.game.send_to_all_active().await;
		trace!("Common game ready waiting");
		self.game.wait_for_all_active().await;
		trace!("Common game ready");
	}

	pub(super) async fn question(&self) {
		let mut qh = QuestionHandler::new(self.game.arc_clone(), QuestionHandlerType::Battle).await;
		qh.handle_all().await;
	}

	pub(super) async fn optional_tip_question(&self) {
		let mut th = TipHandler::new(self.game.arc_clone(), TipHandlerType::Battle).await;
		th.handle_all().await;
	}

	pub(super) async fn send_updated_state(&self) {
		self.game.wait_for_all_active().await;
	}

	async fn new_area_selected(&self, selected_area: u8, player: PlayerName) {
		self.game
			.write()
			.await
			.state
			.selection
			.add_selection(player, County::try_from(selected_area).unwrap());
	}
}
