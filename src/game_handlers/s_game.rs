use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};

use crate::game_handlers::area_conquer_handler::AreaConquerHandler;
use crate::game_handlers::base_handler::BaseHandler;
use crate::game_handlers::battle_handler::BattleHandler;
use crate::game_handlers::fill_remaining_handler::FillRemainingHandler;
use crate::game_handlers::PlayerType;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::war_order::WarOrder;

pub(crate) struct SGame {
	game: SharedTrivGame,
	pub(crate) players: Vec<SGamePlayer>,
}

mod emulation_config {
	pub(crate) const BASE_SELECTION: bool = true;
	pub(crate) const AREA_SELECTION: bool = true;
	pub(crate) const FILL_REMAINING: bool = false;
	pub(crate) const BATTLE: bool = false;
}

impl SGame {
	const PLAYER_COUNT: usize = 3;

	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> SGame {
		SGame {
			game: game.arc_clone(),
			players,
		}
	}

	pub(super) async fn handle_all(&mut self) {
		self.setup().await;
		self.base_selection().await;
		self.area_selection().await;
		self.fill_remaining().await;
		self.battle().await;
	}

	async fn setup(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 11,
			round: 0,
			phase: 0,
		};
		// this must be sent from here as the initial listen state is false
		self.game.send_to_all_players(&self.players).await;
		self.game.wait_for_all_players(&self.players).await;
	}

	async fn base_selection(&self) {
		if emulation_config::BASE_SELECTION {
			SGameStateEmulator::base_selection(self.game.arc_clone()).await;
		} else {
			let base_handler = BaseHandler::new(self.game.arc_clone(), self.players.clone());
			// announcement for players
			self.game.write().await.state.active_player = None;
			base_handler.announcement().await;
			// pick a base for everyone
			for player in &self.players {
				self.game.write().await.state.active_player = Some(player.clone());
				base_handler.start_selection().await;
				base_handler.selection_response().await;
			}
			self.game.write().await.state.selection.clear();
		}
	}

	async fn area_selection(&self) {
		if emulation_config::AREA_SELECTION {
			SGameStateEmulator::area_selection(self.game.arc_clone()).await;
		} else {
			let area_handler = AreaConquerHandler::new(self.game.arc_clone(), self.players.clone());
			let wo = Some(WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT));
			self.game.write().await.state.war_order = wo.clone();
			// setup area handler
			area_handler.setup().await;
			let mut mini_phase_counter = 0;
			// todo change the round count based on the right answers
			for _ in 1..=5 {
				// announcement for all players
				area_handler.announcement().await;
				self.game.write().await.state.round_info = RoundInfo {
					mini_phase_num: 0,
					rel_player_id: 0,
					attacked_player: None,
				};
				// select an area for everyone
				for rel_player in wo
					.clone()
					.unwrap()
					.get_next_players(mini_phase_counter, Self::PLAYER_COUNT)
					.unwrap()
				{
					// todo unify
					self.game.write().await.state.active_player =
						Some(get_player_by_rel_id(self.players.clone(), rel_player));
					area_handler.ask_desired_area().await;
					area_handler.desired_area_response().await;
				}
				area_handler.question().await;
				area_handler.send_updated_state().await;
				let mut game_writer = self.game.write().await;
				game_writer.state.game_state.round += 1;
				game_writer.state.round_info.mini_phase_num = 1;
				mini_phase_counter += Self::PLAYER_COUNT;
			}
		}
	}

	async fn fill_remaining(&mut self) {
		if emulation_config::FILL_REMAINING {
			SGameStateEmulator::fill_remaining(self.game.arc_clone()).await;
		} else {
			let mut fill_remaining_handler =
				FillRemainingHandler::new(self.game.arc_clone(), self.players.clone());
			// setup
			fill_remaining_handler.setup().await;
			// todo improve constant write() calls
			// while there are free areas fill them
			while !self.game.read().await.state.available_areas.is_empty() {
				self.game.write().await.state.round_info.mini_phase_num += 1;
				// announcement for players
				fill_remaining_handler.announcement().await;
				// tip question
				fill_remaining_handler.tip_question().await;
				fill_remaining_handler.ask_desired_area().await;
				fill_remaining_handler.desired_area_response().await;
				let mut write_game = self.game.write().await;
				write_game.state.game_state.round += 1;
				write_game.state.selection.clear();
			}
		}
	}

	async fn battle(&mut self) {
		if emulation_config::BATTLE {
			todo!("add battle emu");
		} else {
			let mut battle_handler =
				BattleHandler::new(self.game.arc_clone(), self.players.clone());
			// let wo = WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT);
			let wo = WarOrder::from(vec![1, 2, 3, 3, 2, 1]);
			self.game.write().await.state.war_order = Some(wo.clone());

			// setup battle handler
			self.game.write().await.state.active_player = None;
			battle_handler.setup().await;
			let mut mini_phase_counter = 0;
			for _ in 0..6 {
				self.game.write().await.state.round_info = RoundInfo {
					mini_phase_num: 0,
					rel_player_id: *wo.get_next_players(0, 1).unwrap().first().unwrap(),
					attacked_player: Some(0),
				};
				// announcement for all players
				battle_handler.announcement().await;

				// let everyone attack
				for rel_player in wo
					.get_next_players(mini_phase_counter, Self::PLAYER_COUNT)
					.unwrap()
				{
					let player = get_player_by_rel_id(self.players.clone(), rel_player);
					self.game.write().await.state.active_player = Some(player);
					battle_handler.ask_attacking_area().await;
					battle_handler.attacked_area_response().await;
					battle_handler.question().await;
					battle_handler.optional_tip_question().await;
				}

				battle_handler.send_updated_state().await;
				self.game.write().await.state.game_state.round += 1;
				self.game.write().await.state.round_info.mini_phase_num = 1;
				mini_phase_counter += Self::PLAYER_COUNT;
			}
		}
	}
}

// Setup,
// BaseSelection,
// AreaSelection,
// FillRemaining,
// Battle,
// EndScreen,

struct SGameStateEmulator {}

impl SGameStateEmulator {
	pub(super) async fn base_selection(game: SharedTrivGame) {
		game.write().await.state.available_areas = AvailableAreas::all_counties();

		let emu_players = vec![
			SGamePlayer::new(PlayerType::Bot, 1, 1),
			SGamePlayer::new(PlayerType::Bot, 2, 2),
			SGamePlayer::new(PlayerType::Bot, 3, 3),
		];
		let bh = BaseHandler::new(game.arc_clone(), emu_players);
		bh.new_base_selected(1, 1).await;
		bh.new_base_selected(8, 2).await;
		bh.new_base_selected(11, 3).await;
	}

	pub(super) async fn area_selection(game: SharedTrivGame) {
		let mut rng = StdRng::from_entropy();
		// this is useful for fill_remaining debugging
		// let round_num = rng.gen_range(1..=5);
		for _ in 1..=5 {
			for rel_player_id in 1..=3 {
				let avail = &game.read().await.state.available_areas.clone();

				let county = *avail.counties().iter().choose(&mut rng).unwrap();
				Area::area_occupied(game.arc_clone(), rel_player_id, Option::from(county))
					.await
					.unwrap();
				game.write().await.state.available_areas.pop_county(&county);
			}
		}
	}

	pub(super) async fn fill_remaining(game: SharedTrivGame) {
		loop {
			let avail = game.read().await.state.available_areas.clone();

			if avail.is_empty() {
				break;
			}

			let mut rng = StdRng::from_entropy();
			let area = *avail.counties().iter().choose(&mut rng).unwrap();
			Area::area_occupied(game.arc_clone(), rng.gen_range(1..3), Option::from(area))
				.await
				.unwrap();
			game.write().await.state.available_areas.pop_county(&area);
		}
	}
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub(crate) struct SGamePlayer {
	player_type: PlayerType,
	// todo remove pub
	pub(crate) id: i32,
	pub(crate) rel_id: u8,
}

impl SGamePlayer {
	pub(crate) fn new(player_type: PlayerType, id: i32, rel_id: u8) -> SGamePlayer {
		SGamePlayer {
			player_type,
			id,
			rel_id,
		}
	}

	pub(crate) fn is_player(&self) -> bool {
		self.player_type == PlayerType::Player
	}
}

// it would be ideal to use a hashset instead of this
pub(crate) fn get_player_by_rel_id(players: Vec<SGamePlayer>, rel_id: u8) -> SGamePlayer {
	players.iter().find(|x| x.rel_id == rel_id).unwrap().clone()
}
