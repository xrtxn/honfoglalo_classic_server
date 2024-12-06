use std::time::Duration;

use fred::clients::RedisPool;
use fred::prelude::KeysInterface;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handlers::question_handler::{QuestionHandler, QuestionHandlerType};
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{player_timeout_timer, send_player_commongame, wait_for_game_ready};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::users::{ServerCommand, User};

#[derive(PartialEq, Clone, Debug)]
enum AreaConquerHandlerPhases {
	// invisible
	Setup,
	// 2,1,0
	Announcement,
	// 2,1,1
	AskDesiredArea,
	// 2,1,3
	DesiredAreaResponse,
	// 2,1,4
	Question,
	// 2,1,7
	SendUpdatedState,
}

impl AreaConquerHandlerPhases {
	fn new() -> AreaConquerHandlerPhases {
		AreaConquerHandlerPhases::Announcement
	}

	fn next(&mut self) {
		match self {
			AreaConquerHandlerPhases::Setup => *self = AreaConquerHandlerPhases::Announcement,
			AreaConquerHandlerPhases::Announcement => {
				*self = AreaConquerHandlerPhases::AskDesiredArea
			}
			AreaConquerHandlerPhases::AskDesiredArea => {
				*self = AreaConquerHandlerPhases::DesiredAreaResponse
			}
			AreaConquerHandlerPhases::DesiredAreaResponse => {
				*self = AreaConquerHandlerPhases::Question
			}
			AreaConquerHandlerPhases::Question => {
				*self = AreaConquerHandlerPhases::SendUpdatedState
			}
			AreaConquerHandlerPhases::SendUpdatedState => {
				*self = {
					error!("Overstepped the phases, returning to AskDesiredArea");
					AreaConquerHandlerPhases::AskDesiredArea
				}
			}
		}
	}
}

pub(crate) struct AreaConquerHandler {
	state: AreaConquerHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl AreaConquerHandler {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> AreaConquerHandler {
		AreaConquerHandler {
			state: AreaConquerHandlerPhases::Setup,
			players,
			game_id,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = AreaConquerHandlerPhases::Announcement;
	}

	pub(crate) fn new_player_pick(&mut self) {
		self.state = AreaConquerHandlerPhases::AskDesiredArea;
	}

	pub(crate) async fn command(
		&mut self,
		temp_pool: &RedisPool,
		active_player: Option<SGamePlayer>,
	) {
		match self.state {
			AreaConquerHandlerPhases::Setup => {
				Self::area_select_setup(temp_pool, self.game_id)
					.await
					.unwrap();
			}
			AreaConquerHandlerPhases::Announcement => {
				GameState::set_phase(temp_pool, self.game_id, 0)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
				}
				trace!("Area select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Area select announcement game ready");
			}
			AreaConquerHandlerPhases::AskDesiredArea => {
				let active_player = active_player.unwrap();
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
				Self::player_area_select_backend(temp_pool, self.game_id, active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					let available = AvailableAreas::get_limited_available(
						temp_pool,
						self.game_id,
						active_player.rel_id,
					)
					.await;
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
			AreaConquerHandlerPhases::DesiredAreaResponse => {
				let active_player = active_player.unwrap();
				if active_player.is_player() {
					player_timeout_timer(temp_pool, active_player.id, Duration::from_secs(60))
						.await;
					Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();

					match User::get_server_command(temp_pool, active_player.id)
						.await
						.unwrap()
					{
						ServerCommand::SelectArea(val) => {
							AreaConquerHandler::new_area_selected(
								temp_pool,
								self.game_id,
								val,
								active_player.rel_id,
							)
							.await
							.unwrap();
						}
						_ => {
							warn!("Invalid command");
						}
					}
				} else {
					let available_areas = AvailableAreas::get_limited_available(
						temp_pool,
						self.game_id,
						active_player.rel_id,
					)
					.await
					.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					AreaConquerHandler::new_area_selected(
						temp_pool,
						self.game_id,
						random_area as u8,
						active_player.rel_id,
					)
					.await
					.unwrap();
				}
				AreaConquerHandler::area_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id, player.rel_id).await;
				}
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
			AreaConquerHandlerPhases::Question => {
				let mut qh = QuestionHandler::new(
					QuestionHandlerType::AreaConquer,
					self.players.clone(),
					self.game_id,
				)
				.await;
				qh.handle_all(temp_pool).await;
			}
			AreaConquerHandlerPhases::SendUpdatedState => {
				// it actually gets sent in the question handler
				wait_for_game_ready(temp_pool, 1).await;
			}
		}
	}

	async fn area_select_setup(temp_pool: &RedisPool, game_id: u32) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 2,
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
