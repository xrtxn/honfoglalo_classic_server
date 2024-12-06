use std::time::Duration;

use fred::clients::RedisPool;
use fred::prelude::KeysInterface;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handlers::question_handler::{TipHandler, TipHandlerType};
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{player_timeout_timer, send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::triviador_state::TriviadorState;
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
	winner: Option<SGamePlayer>,
}

impl FillRemainingHandler {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> FillRemainingHandler {
		FillRemainingHandler {
			state: FillRemainingHandlerPhases::Setup,
			players,
			game_id,
			winner: None,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = FillRemainingHandlerPhases::Announcement;
	}

	pub(crate) async fn setup(&mut self, temp_pool: &RedisPool) {
		Self::fill_setup(temp_pool, self.game_id).await.unwrap();
	}

	pub(crate) async fn announcement(&mut self, temp_pool: &RedisPool) {
		GameState::set_phase(temp_pool, self.game_id, 0)
			.await
			.unwrap();
		for player in self.players.iter().filter(|x| x.is_player()) {
			send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
		}
		trace!("Fill remaining announcement waiting");
		wait_for_game_ready(temp_pool, 1).await;
		trace!("Fill remaining announcement game ready");
	}

	pub(crate) async fn tip_question(&mut self, temp_pool: &RedisPool) {
		let mut th =
			TipHandler::new(TipHandlerType::Fill, self.players.clone(), self.game_id).await;
		self.winner = Some(th.handle_all(temp_pool).await);
	}

	pub(crate) async fn ask_desired_area(&mut self, temp_pool: &RedisPool) {
		let active_player = self.winner.clone().unwrap();
		temp_pool
			.set::<String, _, _>(
				format!("games:{}:send_player", self.game_id),
				active_player.rel_id,
				None,
				None,
				false,
			)
			.await
			.unwrap();
		Self::fill_area_select_backend(temp_pool, self.game_id, active_player.rel_id)
			.await
			.unwrap();
		if active_player.is_player() {
			let available = AvailableAreas::get_available(temp_pool, self.game_id).await;
			Cmd::set_player_cmd(
				temp_pool,
				active_player.id,
				Cmd::select_command(available, 90),
			)
			.await
			.unwrap();
		}
		for player in self.players.iter().filter(|x| x.is_player()) {
			send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
		}
		trace!("Send select cmd waiting");
		wait_for_game_ready(temp_pool, 1).await;
		trace!("Send select cmd game ready");
	}

	pub(crate) async fn desired_area_response(&mut self, temp_pool: &RedisPool) {
		GameState::set_phase(temp_pool, self.game_id, 6)
			.await
			.unwrap();
		let active_player = self.winner.clone().unwrap();
		if active_player.is_player() {
			player_timeout_timer(temp_pool, active_player.id, Duration::from_secs(60)).await;
			Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();

			match User::get_server_command(temp_pool, active_player.id)
				.await
				.unwrap()
			{
				ServerCommand::SelectArea(val) => {
					Self::new_area_selected(temp_pool, self.game_id, val, active_player.rel_id)
						.await
						.unwrap();
				}
				_ => {
					warn!("Invalid command");
				}
			}
		} else {
			let available_areas = AvailableAreas::get_available(temp_pool, self.game_id)
				.await
				.unwrap();

			let mut rng = StdRng::from_entropy();
			let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
			Self::new_area_selected(
				temp_pool,
				self.game_id,
				random_area as u8,
				active_player.rel_id,
			)
			.await
			.unwrap();
		}
		for player in self.players.iter().filter(|x| x.is_player()) {
			send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
		}
		RoundInfo::incr_mini_phase(temp_pool, self.game_id, 1)
			.await
			.unwrap();
	}

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

	async fn fill_area_select_backend(
		temp_pool: &RedisPool,
		game_id: u32,
		winner_rel_id: u8,
	) -> Result<(), anyhow::Error> {
		GameState::set_phase(temp_pool, game_id, 4).await?;

		let ri = RoundInfo::get_roundinfo(temp_pool, game_id).await?;

		let _ = RoundInfo::set_roundinfo(
			temp_pool,
			game_id,
			RoundInfo {
				mini_phase_num: ri.mini_phase_num,
				rel_player_id: winner_rel_id,
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
		Area::area_occupied(
			temp_pool,
			game_id,
			game_player_id,
			County::try_from(selected_area).ok(),
		)
		.await?;

		let mut prev = Selection::get_redis(temp_pool, game_id).await?;
		prev.add_selection(
			PlayerNames::try_from(game_player_id)?,
			County::try_from(selected_area)?,
		);
		let res = Selection::set_redis(temp_pool, game_id, prev).await?;

		Ok(res)
	}

	pub(crate) async fn incr_fill_round(
		temp_pool: &RedisPool,
		game_id: u32,
		by: u8,
	) -> Result<(), anyhow::Error> {
		let mut fill_round = TriviadorState::get_field(temp_pool, game_id, "fill_round")
			.await
			.unwrap_or_else(|_| "0".to_string())
			.parse::<u8>()
			.unwrap_or_else(|_| 0);
		fill_round += by;
		TriviadorState::set_field(
			temp_pool,
			game_id,
			"fill_round",
			fill_round.to_string().as_str(),
		)
		.await?;
		Ok(())
	}
}
