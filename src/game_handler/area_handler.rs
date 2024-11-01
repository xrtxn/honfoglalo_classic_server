use std::time::Duration;

use fred::clients::RedisPool;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::game_handler::question_handler::QuestionHandler;
use crate::game_handler::sgame::SGamePlayer;
use crate::game_handler::{player_timeout_timer, send_player_commongame, wait_for_game_ready};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::users::{ServerCommand, User};

#[derive(PartialEq, Clone)]
enum AreaHandlerPhases {
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

impl AreaHandlerPhases {
	fn new() -> AreaHandlerPhases {
		AreaHandlerPhases::AskDesiredArea
	}

	fn next(&mut self) {
		match self {
			AreaHandlerPhases::Announcement => *self = AreaHandlerPhases::AskDesiredArea,
			AreaHandlerPhases::AskDesiredArea => *self = AreaHandlerPhases::DesiredAreaResponse,
			AreaHandlerPhases::DesiredAreaResponse => *self = AreaHandlerPhases::Question,
			AreaHandlerPhases::Question => *self = AreaHandlerPhases::SendUpdatedState,
			AreaHandlerPhases::SendUpdatedState => {
				*self = {
					error!("Overstepped the phases, returning to AskDesiredArea");
					AreaHandlerPhases::AskDesiredArea
				}
			}
		}
	}
}

pub(crate) struct AreaHandler {
	state: AreaHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl AreaHandler {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> AreaHandler {
		AreaHandler {
			state: AreaHandlerPhases::Announcement,
			players,
			game_id,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_round_pick(&mut self) {
		self.state = AreaHandlerPhases::Announcement;
	}

	pub(crate) fn new_player_pick(&mut self) {
		self.state = AreaHandlerPhases::AskDesiredArea;
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {
		match self.state {
			AreaHandlerPhases::Announcement => {
				let game_state = GameState::get_gamestate(temp_pool, self.game_id)
					.await
					.unwrap();
				if game_state.round == 0 {
					Self::area_select_announcement(temp_pool, self.game_id)
						.await
						.unwrap();
				} else {
					GameState::set_phase(temp_pool, self.game_id, 0)
						.await
						.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Area select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Area select announcement game ready");
			}
			AreaHandlerPhases::AskDesiredArea => {
				Self::player_area_select_backend(temp_pool, self.game_id, active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					let available = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap();
					Cmd::set_player_cmd(
						temp_pool,
						active_player.id,
						Cmd::select_command(available, 90),
					)
					.await
					.unwrap();
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Send select cmd waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Send select cmd game ready");
			}
			AreaHandlerPhases::DesiredAreaResponse => {
				if !active_player.is_player() {
					let available_areas = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap()
						.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					AreaHandler::new_area_selected(
						temp_pool,
						self.game_id,
						random_area as u8,
						active_player.rel_id,
					)
					.await
					.unwrap();
				} else {
					player_timeout_timer(temp_pool, active_player.id, Duration::from_secs(60))
						.await;
					Cmd::clear_cmd(temp_pool, active_player.id).await.unwrap();

					match User::get_server_command(temp_pool, active_player.id)
						.await
						.unwrap()
					{
						ServerCommand::SelectArea(val) => {
							AreaHandler::new_area_selected(
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
				}
				AreaHandler::area_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
			AreaHandlerPhases::Question => {
				let mut qh = QuestionHandler::new(self.players.clone(), self.game_id).await;
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
				qh.next();
				qh.command(temp_pool).await;
			}
			AreaHandlerPhases::SendUpdatedState => {
				trace!("Sendupdatedstate waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Sendupdatedstate game ready");
			}
		}
	}

	async fn area_select_announcement(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		// let _ = join!(
		// 	GameState::set_state(temp_pool, game_id, 2),
		// 	GameState::set_phase(temp_pool, game_id, 0),
		// );
		// if GameState::get_gamestate(temp_pool, game_id).await?.round == 0 {
		// 	GameState::set_round(temp_pool, game_id, 1).await?;
		// }
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

		res += RoundInfo::set_roundinfo(
			temp_pool,
			game_id,
			RoundInfo {
				last_player: game_player_id,
				next_player: game_player_id,
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
