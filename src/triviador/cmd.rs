use serde::Serialize;
use serde_with::skip_serializing_none;

use super::game::{SharedTrivGame, TriviadorGame};
use crate::game_handlers::s_game::SGamePlayer;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::county::available_serialize;

#[skip_serializing_none]
#[derive(Debug, Serialize, Clone)]
pub struct Cmd {
	#[serde(rename = "@CMD")]
	pub command: String,
	#[serde(rename = "@AVAILABLE", serialize_with = "available_serialize")]
	pub available: Option<AvailableAreas>,
	#[serde(rename = "@TO")]
	// seconds for action
	pub timeout: u8,
}

impl Cmd {
	pub fn select_command(available_areas: Option<AvailableAreas>, timeout: u8) -> Cmd {
		Cmd {
			command: "SELECT".to_string(),
			available: available_areas,
			timeout,
		}
	}
	pub fn answer_command(timeout: u8) -> Cmd {
		Cmd {
			command: "ANSWER".to_string(),
			available: None,
			timeout,
		}
	}

	pub fn tip_command(timeout: u8) -> Cmd {
		Cmd {
			command: "TIP".to_string(),
			available: None,
			timeout,
		}
	}

	pub async fn set_player_cmd(game: SharedTrivGame, player_id: &SGamePlayer, cmd: Option<Cmd>) {
		game.write().await.utils.get_mut(&player_id).map(|x| x.cmd = cmd);
	}
}
