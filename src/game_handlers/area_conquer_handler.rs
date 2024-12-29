use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use tracing::{error, trace, warn};

use crate::app::{ServerCommandChannel, XmlPlayerChannel};
use crate::game_handlers::question_handler::{QuestionHandler, QuestionHandlerType};
use crate::game_handlers::s_game::SGamePlayer;
use crate::game_handlers::{send_player_commongame, wait_for_game_ready};
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::county::County;
use crate::triviador::game::{SharedTrivGame, TriviadorGame};
use crate::triviador::game_player_data::PlayerNames;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::users::ServerCommand;

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
	game: SharedTrivGame,
	state: AreaConquerHandlerPhases,
	players: Vec<SGamePlayer>,
}

impl AreaConquerHandler {
	pub(crate) fn new(game: SharedTrivGame, players: Vec<SGamePlayer>) -> AreaConquerHandler {
		AreaConquerHandler {
			game,
			state: AreaConquerHandlerPhases::Setup,
			players,
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

	pub(crate) async fn command(&mut self) {
		match self.state {
			AreaConquerHandlerPhases::Setup => {
				self.area_select_setup().await.unwrap();
			}
			AreaConquerHandlerPhases::Announcement => {
				self.game.write().await.state.game_state.phase = 0;
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(self.game.clone(), player).await;
				}
				trace!("Area select announcement waiting");
				self.game
					.read()
					.await
					.wait_for_all_players(&self.players)
					.await;
				trace!("Area select announcement game ready");
			}
			AreaConquerHandlerPhases::AskDesiredArea => {
				let active_player = self.game.read().await.state.active_player.clone().unwrap();
				let areas = self.game.read().await.state.areas_info.clone();
				trace!("areas: {:?}", areas);
				let available = AvailableAreas::get_limited_available(&areas, active_player.rel_id);
				self.game.write().await.state.available_areas = available.clone();
				self.player_area_select_backend(active_player.rel_id)
					.await
					.unwrap();
				if active_player.is_player() {
					Cmd::set_player_cmd(
						self.game.clone(),
						&active_player,
						Some(Cmd::select_command(available, 90)),
					)
					.await;
				}
				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(self.game.clone(), player).await;
				}
				trace!("Send select cmd waiting");
				self.game
					.read()
					.await
					.wait_for_all_players(&self.players)
					.await;
				trace!("Send select cmd game ready");
			}
			AreaConquerHandlerPhases::DesiredAreaResponse => {
				let active_player = self.game.read().await.state.active_player.clone().unwrap();
				if active_player.is_player() {
					trace!("game acquired");
					// FIXME deadlock
					trace!("Waiting for command channel");
					match self.game.recv_command_channel(&active_player).await.unwrap() {
						ServerCommand::SelectArea(val) => {
							self.new_area_selected(val, active_player.rel_id)
								.await
								.unwrap();
						}
						_ => {
							warn!("Invalid command");
						}
					}
					trace!("command received");
				} else {
					let areas = self.game.read().await.state.areas_info.clone();
					let available_areas =
						AvailableAreas::get_limited_available(&areas, active_player.rel_id)
							.unwrap();

					let mut rng = StdRng::from_entropy();
					let random_area = available_areas.areas.into_iter().choose(&mut rng).unwrap();
					self.new_area_selected(random_area as u8, active_player.rel_id)
						.await
						.unwrap();
				}
				self.area_selected_stage().await;

				for player in self.players.iter().filter(|x| x.is_player()) {
					send_player_commongame(self.game.clone(), player).await;
				}
				trace!("Common game ready waiting");
				self.game
					.read()
					.await
					.wait_for_all_players(&self.players)
					.await;
				trace!("Common game ready");
			}
			AreaConquerHandlerPhases::Question => {
				let mut qh = QuestionHandler::new(
					self.game.clone(),
					QuestionHandlerType::AreaConquer,
					self.players.clone(),
				)
				.await;
				qh.handle_all().await;
			}
			AreaConquerHandlerPhases::SendUpdatedState => {
				// it actually gets sent in the question handler
				self.game
					.read()
					.await
					.wait_for_all_players(&self.players)
					.await;
			}
		}
	}

	async fn area_select_setup(&self) -> Result<(), anyhow::Error> {
		self.game.write().await.state.game_state = GameState {
			state: 2,
			round: 1,
			phase: 0,
		};
		Ok(())
	}

	async fn player_area_select_backend(&self, game_player_id: u8) -> Result<(), anyhow::Error> {
		let mut game = self.game.write().await;
		game.state.game_state.phase = 1;

		game.state.round_info = RoundInfo {
			mini_phase_num: game.state.round_info.mini_phase_num + 1,
			rel_player_id: game_player_id,
			attacked_player: None,
		};
		Ok(())
	}

	pub async fn area_selected_stage(&self) {
		// sets phase to 3
		self.game.write().await.state.game_state.phase = 3;
	}

	pub async fn new_area_selected(
		&self,
		selected_area: u8,
		game_player_id: u8,
	) -> Result<(), anyhow::Error> {
		AvailableAreas::pop_county(self.game.clone(), County::try_from(selected_area)?).await;

		// let mut prev = Selection::get_redis(temp_pool, game_id).await?;
		self.game.write().await.state.selection.add_selection(
			PlayerNames::try_from(game_player_id)?,
			County::try_from(selected_area)?,
		);
		Ok(())
	}
}
