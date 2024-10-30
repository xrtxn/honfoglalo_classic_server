use fred::clients::RedisPool;
use tracing::trace;

use crate::game_handler::area_handler::AreaHandler;
use crate::game_handler::base_handler::BaseHandler;
use crate::game_handler::{send_player_commongame, wait_for_game_ready, PlayerType};
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;

pub(crate) struct SGame {
	game_state: SGameState,
	base_handler: BaseHandler,
	area_handler: AreaHandler,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl SGame {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> SGame {
		SGame {
			game_state: SGameState::new(),
			base_handler: BaseHandler::new(players.clone(), game_id),
			area_handler: AreaHandler::new(players.clone(), game_id),
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
				for _ in 0..6 {
					// announcement for all players
					for player in self.players.iter().filter(|x| x.is_player()) {
						self.area_handler.new_round_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
					}
					// select an area for everyone
					for player in self.players.iter() {
						self.area_handler.new_player_pick();
						self.area_handler.command(temp_pool, player.clone()).await;
						self.area_handler.next();
						self.area_handler.command(temp_pool, player.clone()).await;
					}
					self.area_handler.next();
					self.area_handler
						.command(temp_pool, self.players[0].clone())
						.await;
					GameState::incr_round(temp_pool, self.game_id, 1)
						.await
						.unwrap();
					RoundInfo::set_roundinfo(
						temp_pool,
						self.game_id,
						RoundInfo {
							last_player: 1,
							next_player: 1,
						},
					)
					.await
					.unwrap();
				}
			}
			SGameState::Battle => {
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
	Battle,
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
			SGameState::Battle => {
				todo!("Implement next phase")
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
