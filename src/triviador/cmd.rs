use serde::Serialize;
use serde_with::skip_serializing_none;

use super::{game::SharedTrivGame, game_player_data::PlayerName};
use crate::triviador::available_area::AvailableAreas;

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct Cmd {
	#[serde(rename = "@CMD")]
	pub command: String,
	#[serde(
		rename = "@AVAILABLE",
		skip_serializing_if = "AvailableAreas::is_empty"
	)]
	pub available: AvailableAreas,
	#[serde(rename = "@TO")]
	// seconds for action
	pub timeout: u8,
}

impl Cmd {
	pub(crate) fn select_command(available_areas: AvailableAreas, timeout: u8) -> Cmd {
		Cmd {
			command: "SELECT".to_string(),
			available: available_areas,
			timeout,
		}
	}
	pub(crate) fn answer_command(timeout: u8) -> Cmd {
		Cmd {
			command: "ANSWER".to_string(),
			available: AvailableAreas::new(),
			timeout,
		}
	}
	pub(crate) fn tip_command(timeout: u8) -> Cmd {
		Cmd {
			command: "TIP".to_string(),
			available: AvailableAreas::new(),
			timeout,
		}
	}

	pub(crate) async fn set_player_cmd(
		game: SharedTrivGame,
		player_id: &PlayerName,
		cmd: Option<Cmd>,
	) {
		game.write()
			.await
			.utils
			.get_player_mut(player_id)
			.unwrap()
			.set_cmd(cmd);
	}
}
