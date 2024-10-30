use std::time::Duration;

use fred::clients::RedisPool;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{trace, warn};

use crate::game_handler::sgame::SGamePlayer;
use crate::game_handler::{player_timeout_timer, send_player_commongame, wait_for_game_ready};
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::{Base, Bases};
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::triviador_state::TriviadorState;
use crate::users::{ServerCommand, User};

#[derive(PartialEq, Clone)]
enum BaseHandlerPhases {
	Announcement,
	StartSelection,
	SelectionResponse,
}

impl BaseHandlerPhases {
	fn new() -> BaseHandlerPhases {
		BaseHandlerPhases::StartSelection
	}

	fn next(&mut self) {
		*self = match self {
			BaseHandlerPhases::Announcement => BaseHandlerPhases::StartSelection,
			BaseHandlerPhases::StartSelection => BaseHandlerPhases::SelectionResponse,
			BaseHandlerPhases::SelectionResponse => BaseHandlerPhases::Announcement,
		}
	}
}

pub(crate) struct BaseHandler {
	state: BaseHandlerPhases,
	players: Vec<SGamePlayer>,
	game_id: u32,
}

impl BaseHandler {
	pub(crate) fn new(players: Vec<SGamePlayer>, game_id: u32) -> BaseHandler {
		BaseHandler {
			state: BaseHandlerPhases::Announcement,
			players,
			game_id,
		}
	}

	pub(crate) fn next(&mut self) {
		self.state.next();
	}

	pub(crate) fn new_pick(&mut self) {
		self.state = BaseHandlerPhases::StartSelection;
	}

	pub(crate) async fn command(&mut self, temp_pool: &RedisPool, active_player: SGamePlayer) {
		match self.state {
			BaseHandlerPhases::Announcement => {
				Self::base_select_announcement(temp_pool, self.game_id)
					.await
					.unwrap();
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Base select announcement waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Base select announcement game ready");
				AvailableAreas::set_available(
					temp_pool,
					self.game_id,
					AvailableAreas::all_counties(),
				)
				.await
				.unwrap();
			}
			BaseHandlerPhases::StartSelection => {
				Self::player_base_select_backend(temp_pool, self.game_id, active_player.rel_id)
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
			BaseHandlerPhases::SelectionResponse => {
				if !active_player.is_player() {
					let available_areas = AvailableAreas::get_available(temp_pool, self.game_id)
						.await
						.unwrap()
						.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					BaseHandler::new_base_selected(
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
							BaseHandler::new_base_selected(
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
				BaseHandler::base_selected_stage(temp_pool, self.game_id)
					.await
					.unwrap();

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(temp_pool, self.game_id, player.id).await;
				}
				trace!("Common game ready waiting");
				wait_for_game_ready(temp_pool, 1).await;
				trace!("Common game ready");
			}
		}
	}

	pub async fn base_selected_stage(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		let res: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 1,
				round: 0,
				phase: 3,
			},
		)
		.await?;
		Ok(res)
	}

	pub async fn new_base_selected(
		temp_pool: &RedisPool,
		game_id: u32,
		selected_area: u8,
		rel_id: u8,
	) -> Result<u8, anyhow::Error> {
		AvailableAreas::pop_county(temp_pool, game_id, County::try_from(selected_area)?).await?;

		Bases::add_base(
			temp_pool,
			game_id,
			PlayerNames::try_from(rel_id)?,
			Base::new(selected_area),
		)
		.await?;

		Area::base_selected(temp_pool, game_id, rel_id, County::try_from(selected_area)?).await?;

		let res = TriviadorState::set_field(
			temp_pool,
			game_id,
			"selection",
			&Bases::serialize_full(&Bases::get_redis(temp_pool, game_id).await?)?,
		)
		.await?;
		TriviadorState::modify_player_score(temp_pool, game_id, rel_id - 1, 1000).await?;
		Ok(res)
	}

	async fn base_select_announcement(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<(), anyhow::Error> {
		let _: u8 = GameState::set_gamestate(
			temp_pool,
			game_id,
			GameState {
				state: 1,
				round: 0,
				phase: 0,
			},
		)
		.await?;
		Ok(())
	}

	async fn player_base_select_backend(
		temp_pool: &RedisPool,
		game_id: u32,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		let mut res: u8 = GameState::set_phase(temp_pool, game_id, 1).await?;

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
}
