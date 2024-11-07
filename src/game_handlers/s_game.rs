use fred::clients::RedisPool;
use tracing::{error, trace};

use crate::game_handlers::area_handler::AreaHandler;
use crate::game_handlers::base_handler::BaseHandler;
use crate::game_handlers::battle_handler::BattleHandler;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready, PlayerType};
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::war_order::WarOrder;

pub(crate) struct SGame {
	game_state: SGameState,
	base_handler: BaseHandler,
	area_handler: AreaHandler,
	battle_handler: BattleHandler,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl SGame {
	const PLAYER_COUNT: usize = 3;

	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> SGame {
		SGame {
			game_state: SGameState::new(),
			base_handler: BaseHandler::new(players.clone(), game_id),
			area_handler: AreaHandler::new(players.clone(), game_id),
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
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Setup waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Setup game ready");
			}
			SGameState::BaseSelection => {
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
			SGameState::AreaSelection => {
				RoundInfo::set_roundinfo(
					temp_pool,
					self.game_id,
					RoundInfo {
						mini_phase_num: 1,
						rel_player_id: 1,
						attacked_player: None,
					},
				)
				.await
				.unwrap();
				WarOrder::set_redis(
					&WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT),
					temp_pool,
					self.game_id,
				)
				.await
				.unwrap();
				let mut mini_phase_num = 0;
				// todo this is 6
				for _ in 0..1 {
					// announcement for all players
					for player in self.players.iter().filter(|x| x.is_player()) {
						self.area_handler.new_round_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
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
					}
					let war_order = WarOrder::get_redis(temp_pool, self.game_id).await.unwrap();
					// select an area for everyone
					for rel_player in war_order
						.get_next_players(mini_phase_num, Self::PLAYER_COUNT)
						.unwrap()
					{
						let player = self.get_player_by_rel_id(rel_player);
						self.area_handler.new_player_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
						self.area_handler.next();
						self.area_handler.command(temp_pool, player).await;
					}
					self.area_handler.next();
					self.area_handler
						.command(temp_pool, self.players[0].clone())
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
			SGameState::Battle => {
				// todo this is 6
				for _ in 0..2 {
					GameState::set_round(temp_pool, self.game_id, 0)
						.await
						.unwrap();
					// announcement for all players
					for player in self.players.iter().filter(|x| x.is_player()) {
						self.battle_handler.new_round_pick();
						self.battle_handler.command(temp_pool, player.clone()).await;
					}
					// select an area for everyone
					for player in self.players.iter() {
						self.battle_handler.new_player_pick();
						self.battle_handler.command(temp_pool, player.clone()).await;
						self.battle_handler.next();
						self.battle_handler.command(temp_pool, player.clone()).await;
					}
					self.battle_handler.next();
					self.battle_handler
						.command(temp_pool, self.players[0].clone())
						.await;
					GameState::incr_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();
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

	// it would be ideal to use a hashset instead of this
	pub(crate) fn get_player_by_rel_id(&self, rel_id: u8) -> SGamePlayer {
		self.players
			.iter()
			.find(|x| x.rel_id == rel_id)
			.unwrap()
			.clone()
	}
}

#[derive(Clone)]
enum SGameState {
	Setup,
	BaseSelection,
	AreaSelection,
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
			SGameState::AreaSelection => SGameState::Battle,
			SGameState::Battle => SGameState::EndScreen,
			SGameState::EndScreen => {
				error!("Overshot the game state");
				SGameState::Setup
			}
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
