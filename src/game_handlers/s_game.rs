use fred::clients::RedisPool;
use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};
use tracing::{error, trace};

use crate::game_handlers::area_conquer_handler::AreaConquerHandler;
use crate::game_handlers::base_handler::BaseHandler;
use crate::game_handlers::battle_handler::BattleHandler;
use crate::game_handlers::fill_remaining_handler::FillRemainingHandler;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready, PlayerType};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::county::County;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::war_order::WarOrder;

pub(crate) struct SGame {
	game_state: SGameState,
	base_handler: BaseHandler,
	area_handler: AreaConquerHandler,
	fill_remaining_handler: FillRemainingHandler,
	battle_handler: BattleHandler,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

mod emulation_config {
	pub(crate) const BASE_SELECTION: bool = true;
	pub(crate) const AREA_SELECTION: bool = false;
	pub(crate) const FILL_REMAINING: bool = false;
	pub(crate) const BATTLE: bool = false;
}
impl SGame {
	const PLAYER_COUNT: usize = 3;

	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> SGame {
		SGame {
			game_state: SGameState::new(),
			base_handler: BaseHandler::new(players.clone(), game_id),
			area_handler: AreaConquerHandler::new(players.clone(), game_id),
			fill_remaining_handler: FillRemainingHandler::new(players.clone(), game_id),
			battle_handler: BattleHandler::new(players.clone(), game_id),
			players,
			game_id,
		}
	}

	pub(crate) fn next(&mut self) {
		self.game_state = self.game_state.next()
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool) {
		match self.game_state {
			SGameState::Setup => {
				Self::setup_backend(temp_pool, self.game_id).await.unwrap();
				// this must be sent from here as the initial listen state is false
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
				}
				trace!("Setup waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Setup game ready");
			}
			SGameState::BaseSelection => {
				if emulation_config::BASE_SELECTION {
					SGameStateEmulator::base_selection(temp_pool, self.game_id).await;
				} else {
					// announcement for players
					for player in self.players.iter().filter(|x| x.is_player()) {
						self.base_handler.command(temp_pool, player.clone()).await;
					}
					// pick a base for everyone
					for player in &self.players {
						self.base_handler.new_pick();
						self.base_handler.command(temp_pool, player.clone()).await;
						self.base_handler.next();
						self.base_handler.command(temp_pool, player.clone()).await;
					}
					Selection::clear(temp_pool, self.game_id).await.unwrap()
				}
			}
			SGameState::AreaSelection => {
				if emulation_config::AREA_SELECTION {
					SGameStateEmulator::area_selection(temp_pool, self.game_id).await;
				} else {
					WarOrder::set_redis(
						&WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT),
						temp_pool,
						self.game_id,
					)
					.await
					.unwrap();
					// setup area handler
					self.area_handler.command(temp_pool, None).await;
					let mut mini_phase_num = 0;
					for _ in 0..5 {
						// announcement for all players
						self.area_handler.new_round_pick();
						self.area_handler.command(temp_pool, None).await;
						let war_order = WarOrder::get_redis(temp_pool, self.game_id).await.unwrap();
						RoundInfo::set_roundinfo(
							temp_pool,
							self.game_id,
							RoundInfo {
								mini_phase_num: 0,
								rel_player_id: 0,
								attacked_player: None,
							},
						)
						.await
						.unwrap();
						// select an area for everyone
						for rel_player in war_order
							.get_next_players(mini_phase_num, Self::PLAYER_COUNT)
							.unwrap()
						{
							let player = get_player_by_rel_id(self.players.clone(), rel_player);
							self.area_handler.new_player_pick();
							self.area_handler
								.command(temp_pool, Some(player.clone()))
								.await;
							self.area_handler.next();
							self.area_handler.command(temp_pool, Some(player)).await;
						}
						self.area_handler.next();
						self.area_handler
							.command(temp_pool, Some(self.players[0].clone()))
							.await;
						GameState::incr_round(temp_pool, self.game_id, 1)
							.await
							.unwrap();
						RoundInfo::incr_mini_phase(temp_pool, self.game_id, 1)
							.await
							.unwrap();
						mini_phase_num += Self::PLAYER_COUNT;
					}
				}
			}
			SGameState::FillRemaining => {
				if emulation_config::FILL_REMAINING {
					SGameStateEmulator::fill_remaining(temp_pool, self.game_id).await;
				}
				// setup
				self.fill_remaining_handler.setup(temp_pool).await;
				// while there are free areas fill them
				while !AvailableAreas::get_available(temp_pool, self.game_id)
					.await
					.unwrap()
					.areas
					.is_empty()
				{
					FillRemainingHandler::incr_fill_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();

					// announcement for players
					self.fill_remaining_handler.announcement(temp_pool).await;

					// tip question
					self.fill_remaining_handler.tip_question(temp_pool).await;

					self.fill_remaining_handler
						.ask_desired_area(temp_pool)
						.await;

					self.fill_remaining_handler
						.desired_area_response(temp_pool)
						.await;
					GameState::incr_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();
					Selection::clear(temp_pool, self.game_id).await.unwrap()
				}
			}
			SGameState::Battle => {
				WarOrder::set_redis(
					&WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT),
					temp_pool,
					self.game_id,
				)
				.await
				.unwrap();
				// setup battle handler
				self.battle_handler.command(temp_pool, None).await;
				let mut mini_phase_num = 0;
				for _ in 0..6 {
					GameState::set_round(temp_pool, self.game_id, 0)
						.await
						.unwrap();
					RoundInfo::set_roundinfo(
						temp_pool,
						self.game_id,
						RoundInfo {
							mini_phase_num: 0,
							rel_player_id: 0,
							attacked_player: Some(0),
						},
					)
					.await
					.unwrap();
					// announcement for all players
					self.battle_handler.new_round_pick();
					self.battle_handler.command(temp_pool, None).await;

					let war_order = WarOrder::get_redis(temp_pool, self.game_id).await.unwrap();

					// let everyone attack
					for rel_player in war_order
						.get_next_players(mini_phase_num, Self::PLAYER_COUNT)
						.unwrap()
					{
						let player = get_player_by_rel_id(self.players.clone(), rel_player);
						self.battle_handler.new_player_pick();
						self.battle_handler
							.command(temp_pool, Some(player.clone()))
							.await;
						self.battle_handler.next();
						self.battle_handler
							.command(temp_pool, Some(player.clone()))
							.await;
						self.battle_handler.next();
						self.battle_handler
							.command(temp_pool, Some(self.players[0].clone()))
							.await;
						RoundInfo::incr_mini_phase(temp_pool, self.game_id, 1)
							.await
							.unwrap();
					}

					GameState::incr_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();
					mini_phase_num += Self::PLAYER_COUNT;
				}
			}
			SGameState::EndScreen => {
				todo!("Implement next phase")
			}
		}
	}

	async fn setup_backend(temp_pool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 11,
				round: 0,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone)]
enum SGameState {
	Setup,
	BaseSelection,
	AreaSelection,
	FillRemaining,
	Battle,
	EndScreen,
}

impl SGameState {
	fn new() -> SGameState {
		SGameState::Setup
	}

	fn next(&self) -> SGameState {
		match self {
			SGameState::Setup => SGameState::BaseSelection,
			SGameState::BaseSelection => SGameState::AreaSelection,
			SGameState::AreaSelection => SGameState::FillRemaining,
			SGameState::FillRemaining => SGameState::Battle,
			SGameState::Battle => SGameState::EndScreen,
			SGameState::EndScreen => {
				error!("Overshot the game state");
				SGameState::Setup
			}
		}
	}
}

struct SGameStateEmulator {}

impl SGameStateEmulator {
	pub(super) async fn base_selection(temp_pool: &RedisPool, game_id: u32) {
		AvailableAreas::set_available(temp_pool, game_id, AvailableAreas::all_counties())
			.await
			.unwrap();

		BaseHandler::new_base_selected(temp_pool, game_id, 16, 1)
			.await
			.unwrap();
		BaseHandler::new_base_selected(temp_pool, game_id, 15, 2)
			.await
			.unwrap();
		BaseHandler::new_base_selected(temp_pool, game_id, 17, 3)
			.await
			.unwrap();
	}

	pub(super) async fn area_selection(temp_pool: &RedisPool, game_id: u32) {
		for _ in 0..5 {
			for rel_player_id in 1..4 {
				let avail = AvailableAreas::get_available(temp_pool, game_id)
					.await
					.unwrap()
					.areas;

				let mut rng = StdRng::from_entropy();
				let area = avail.iter().choose(&mut rng).unwrap().clone();
				Area::area_occupied(temp_pool, game_id, rel_player_id, Option::from(area))
					.await
					.unwrap();
				AvailableAreas::pop_county(temp_pool, game_id, County::try_from(area).unwrap())
					.await
					.unwrap();
			}
		}
	}

	pub(super) async fn fill_remaining(temp_pool: &RedisPool, game_id: u32) {
		loop {
			let avail = AvailableAreas::get_available(temp_pool, game_id)
				.await
				.unwrap()
				.areas;

			if avail.is_empty() {
				break;
			}

			let mut rng = StdRng::from_entropy();
			let area = avail.iter().choose(&mut rng).unwrap().clone();
			Area::area_occupied(temp_pool, game_id, rng.gen_range(1..3), Option::from(area))
				.await
				.unwrap();
			AvailableAreas::pop_county(temp_pool, game_id, County::try_from(area).unwrap())
				.await
				.unwrap();
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
