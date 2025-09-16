use std::collections::HashMap;

use rand::SeedableRng;
use rand::prelude::{IteratorRandom, StdRng};
use tokio_stream::{Stream, StreamExt};
use tracing::{info, trace, warn};

use crate::game_handlers::area_conquer_handler::AreaConquerHandler;
use crate::game_handlers::base_handler::BaseHandler;
use crate::game_handlers::battle_handler::BattleHandler;
use crate::game_handlers::fill_remaining_handler::FillRemainingHandler;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::cmd::Cmd;
use crate::triviador::game::SharedTrivGame;
use crate::triviador::game_player_data::PlayerName;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::triviador_state::GamePlayerChannels;
use crate::triviador::war_order::WarOrder;

use super::endscreen_handler::EndScreenHandler;

pub(crate) struct SGame {
	game: SharedTrivGame,
	pub(crate) players: GamePlayerInfo,
}

pub(crate) mod emulation_config {
	//todo read from .env or similar
	pub(crate) const BASE_SELECTION: bool = true;
	pub(crate) const AREA_SELECTION: bool = false;
	pub(crate) const FILL_REMAINING: bool = false;
	pub(crate) const BATTLE: bool = false;
}

impl SGame {
	const PLAYER_COUNT: usize = 3;

	pub(crate) fn new(game: SharedTrivGame, players: GamePlayerInfo) -> SGame {
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
		self.end_screen().await;
	}

	async fn setup(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 11,
			round: 0,
			phase: 0,
		};
		// this must be sent from here as the initial listen state is false
		self.game.send_to_all_active().await;
		self.game.wait_for_all_active().await;
	}

	async fn base_selection(&self) {
		trace!("base selection");
		if emulation_config::BASE_SELECTION {
			SGameStateEmulator::base_selection(self.game.arc_clone()).await;
		} else {
			let base_handler = BaseHandler::new(self.game.arc_clone());
			// announcement for players
			self.game.write().await.state.active_player = None;
			base_handler.announcement().await;
			// pick a base for everyone
			for (player, _) in self.players.0.clone() {
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
			let area_handler = AreaConquerHandler::new(self.game.arc_clone());
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
					active_player: PlayerName::Nobody,
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
					self.game.write().await.state.active_player = Some(rel_player);
					area_handler.ask_desired_area().await;
					area_handler.desired_area_response().await;
				}
				area_handler.question().await;
				area_handler.send_updated_state().await;
				let mut game_writer = self.game.write().await;
				game_writer.state.selection.clear();
				game_writer.state.game_state.round += 1;
				game_writer.state.round_info.mini_phase_num = 1;
				drop(game_writer);
				mini_phase_counter += Self::PLAYER_COUNT;
			}
		}
	}

	async fn fill_remaining(&mut self) {
		if emulation_config::FILL_REMAINING {
			SGameStateEmulator::fill_remaining(self.game.arc_clone()).await;
		} else {
			let mut fill_remaining_handler = FillRemainingHandler::new(self.game.arc_clone());
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
			warn!("add battle emu");
		} else {
			let mut battle_handler = BattleHandler::new(self.game.arc_clone());
			// let wo = WarOrder::new_random_with_size(WarOrder::NORMAL_ROUND_COUNT);
			let wo = WarOrder::from(vec![3,2,2, 3, 2, 3, 1, 2, 3, 1, 2, 3]);
			self.game.write().await.state.war_order = Some(wo.clone());

			// setup battle handler
			self.game.write().await.state.active_player = None;
			battle_handler.setup().await;

			self.game.write().await.state.round_info = RoundInfo {
				mini_phase_num: 0,
				active_player: *wo.get_next_players(0, 1).unwrap().first().unwrap(),
				attacked_player: Some(PlayerName::Nobody),
			};
			// announcement for all players
			battle_handler.announcement().await;

			let mut mini_phase_counter = 0;
			'war_loop: for _ in 0..=wo.order.len() / Self::PLAYER_COUNT {
				// let everyone attack in order
				for player in wo
					.get_next_players(mini_phase_counter, Self::PLAYER_COUNT)
					.unwrap()
				{
					// check if only one player is left
					if self.game.read().await.state.eliminated_players.len()
						>= Self::PLAYER_COUNT - 1
					{
						info!("All players are eliminated, ending game");
						break 'war_loop;
					}
					//skip eliminated players
					if !self
						.game
						.read()
						.await
						.state
						.eliminated_players
						.contains(&player)
					{
						let mut game_write = self.game.write().await;
						game_write.state.round_info.mini_phase_num += 1;
						game_write.state.active_player = Some(player);
						drop(game_write);
						battle_handler.handle_attacking().await;
					} else {
						self.game.write().await.state.round_info.mini_phase_num += 1;
					}
				}

				self.game.write().await.state.game_state.round += 1;
				self.game.write().await.state.round_info.mini_phase_num = 0;

				mini_phase_counter += Self::PLAYER_COUNT;
			}
		}
	}

	async fn end_screen(&self) {
		let end_screen_handler = EndScreenHandler::new(self.game.arc_clone());
		end_screen_handler.handle_all().await;
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
		let bh = BaseHandler::new(game.arc_clone());
		bh.new_base_selected(1, PlayerName::Player1).await;
		bh.new_base_selected(8, PlayerName::Player2).await;
		bh.new_base_selected(11, PlayerName::Player3).await;
	}

	pub(super) async fn area_selection(game: SharedTrivGame) {
		let mut rng = StdRng::from_entropy();
		// this is useful for fill_remaining debugging
		// let round_num = rng.gen_range(1..=5);
		for _ in 1..=5 {
			for player in PlayerName::all() {
				let avail = &game.read().await.state.available_areas.clone();

				let county = *avail.counties().iter().choose(&mut rng).unwrap();
				Area::area_occupied(game.arc_clone(), player, Option::from(county))
					.await
					.unwrap();
				game.write().await.state.available_areas.pop_county(&county);
			}
		}
	}

	pub(super) async fn fill_remaining(game: SharedTrivGame) {
		let mut rng = StdRng::from_entropy();
		loop {
			let avail = game.read().await.state.available_areas.clone();

			if avail.is_empty() {
				break;
			}

			let area = *avail.counties().iter().choose(&mut rng).unwrap();
			Area::area_occupied(
				game.arc_clone(),
				PlayerName::all().choose(&mut rng).unwrap(),
				Option::from(area),
			)
			.await
			.unwrap();
			game.write().await.state.available_areas.pop_county(&area);
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct GamePlayerInfo(HashMap<PlayerName, SGamePlayerInfo>);

impl GamePlayerInfo {
	pub(crate) fn new() -> GamePlayerInfo {
		GamePlayerInfo(HashMap::new())
	}

	pub(crate) fn add(&mut self, player: PlayerName, info: SGamePlayerInfo) {
		self.0.insert(player, info);
	}

	pub(crate) fn get_player(&self, player: &PlayerName) -> Option<&SGamePlayerInfo> {
		self.0.get(&player)
	}

	pub(crate) fn get_player_mut(&mut self, player: &PlayerName) -> Option<&mut SGamePlayerInfo> {
		self.0.get_mut(&player)
	}

	pub(crate) fn players_with_info_stream(
		&self,
	) -> impl Stream<Item = (&PlayerName, &SGamePlayerInfo)> + '_ {
		tokio_stream::iter(&self.0)
	}

	#[allow(dead_code)]
	pub(crate) fn players_stream(&self) -> impl Stream<Item = &PlayerName> + '_ {
		tokio_stream::iter(self.0.keys())
	}

	pub(crate) fn active_players_stream(&self) -> impl Stream<Item = &PlayerName> + '_ {
		tokio_stream::iter(&self.0)
			.filter(|(_, info)| info.is_player())
			.map(|(player, _)| player)
	}

	/// Returns a stream of active players (not robots)
	pub(crate) fn active_with_info_stream(
		&self,
	) -> impl Stream<Item = (&PlayerName, &SGamePlayerInfo)> + '_ {
		tokio_stream::iter(&self.0).filter(|(_, info)| info.is_player())
	}

	#[allow(dead_code)]
	/// Returns a stream of inactive players (robots)
	pub(crate) fn inactive_with_info_stream(
		&self,
	) -> impl Stream<Item = (&PlayerName, &SGamePlayerInfo)> + '_ {
		tokio_stream::iter(&self.0).filter(|(_, info)| !info.is_player())
	}
}

impl From<HashMap<PlayerName, SGamePlayerInfo>> for GamePlayerInfo {
	fn from(map: HashMap<PlayerName, SGamePlayerInfo>) -> Self {
		GamePlayerInfo(map)
	}
}

#[derive(Clone, Debug)]
pub(crate) struct SGamePlayerInfo {
	active: bool,
	cmd: Option<Cmd>,
	channels: Option<GamePlayerChannels>,
}

impl SGamePlayerInfo {
	pub(crate) fn new(is_active: bool) -> SGamePlayerInfo {
		SGamePlayerInfo {
			active: is_active,
			cmd: None,
			channels: None,
		}
	}

	pub(crate) fn is_player(&self) -> bool {
		self.active
	}

	#[allow(dead_code)]
	pub(crate) fn set_active(&mut self, active: bool) {
		self.active = active;
	}

	pub(crate) fn set_cmd(&mut self, cmd: Option<Cmd>) {
		self.cmd = cmd;
	}
	pub(crate) fn get_cmd(&self) -> &Option<Cmd> {
		&self.cmd
	}

	pub(crate) fn set_channels(&mut self, channels: Option<GamePlayerChannels>) {
		self.channels = channels;
	}

	pub(crate) fn get_player_channels(&self) -> &Option<GamePlayerChannels> {
		&self.channels
	}
}
