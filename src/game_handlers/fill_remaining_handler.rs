use std::time::Duration;

use fred::clients::RedisPool;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handlers::question_handler::QuestionHandler;
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{player_timeout_timer, send_player_commongame, wait_for_game_ready};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
use crate::triviador::war_order::WarOrder;
use crate::users::{ServerCommand, User};

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
	state: FillRemainingHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl FillRemainingHandler {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> FillRemainingHandler {
		FillRemainingHandler {
			state: FillRemainingHandlerPhases::Announcement,
			players,
			game_id,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = FillRemainingHandlerPhases::Announcement;
	}

	pub(crate) fn new_player_pick(&mut self) {
		self.state = FillRemainingHandlerPhases::TipQuestion;
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {}

	async fn fill_setup(temp_pool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 3,
				round: 1,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}

	async fn player_area_select_backend(
		temp_pool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		// sets phase to 1
		let mut res = GameState::set_phase(temp_pool, game_id, 1).await?;
		let mut ri = RoundInfo::get_roundinfo(temp_pool, game_id).await?;
		ri.mini_phase_num += 1;

		res += RoundInfo::set_roundinfo(
			temp_pool,
			game_id,
			RoundInfo {
				mini_phase_num: ri.mini_phase_num,
				rel_player_id: game_player_id,
				attacked_player: None,
			},
		)
		.await?;
		Ok(())
	}

	pub async fn area_selected_stage(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		// sets phase to 3
		let res: u8 = GameState::incr_phase(temp_pool, game_id, 2).await?;
		Ok(res)
	}

	pub async fn new_area_selected(
		temp_pool: &RedisPool,
		game_id: u32,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;

		let mut prev = Selection::get_redis(temp_pool, game_id).await?;
		prev.add_selection(
			PlayerNames::try_from(game_player_id)?,
			County::try_from(selected_area)?,
		);
		let res = Selection::set_redis(temp_pool, game_id, prev).await?;

		Ok(res)
	}
}
