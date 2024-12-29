use std::collections::HashMap;

use serde::Serialize;
use serde_with::skip_serializing_none;

use super::game::SharedTrivGame;
use super::game::TriviadorGame;
use super::war_order::WarOrder;
use crate::app::ServerCommandChannel;
use crate::app::XmlPlayerChannel;
use crate::game_handlers::s_game::SGamePlayer;
use crate::triviador::areas::areas_full_serializer;
use crate::triviador::areas::Area;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::Bases;
use crate::triviador::county::available_serialize;
use crate::triviador::county::County;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::shield_mission::ShieldMission;
use crate::users::ServerCommand;

#[skip_serializing_none]
#[derive(Serialize, Debug, Clone)]
pub(crate) struct TriviadorState {
	#[serde(rename = "@SCR")]
	pub map_name: String,
	#[serde(rename = "@ST")]
	pub game_state: GameState,
	#[serde(rename = "@CP")]
	pub round_info: RoundInfo,
	#[serde(rename = "@HC")]
	// numbers of players connected e.g. 1,2,3
	pub players_connected: String,
	#[serde(rename = "@CHS")]
	pub players_chat_state: String,
	#[serde(rename = "@PTS")]
	pub players_points: String,
	#[serde(rename = "@SEL")]
	pub selection: Selection,
	#[serde(rename = "@B")]
	pub base_info: Bases,
	#[serde(rename = "@A", serialize_with = "areas_full_serializer")]
	pub areas_info: HashMap<County, Area>,
	#[serde(rename = "@AA", serialize_with = "available_serialize")]
	// todo remove option
	pub available_areas: Option<AvailableAreas>,
	#[serde(rename = "@UH")]
	pub used_helps: String,
	#[serde(rename = "@FAO")]
	pub fill_round: Option<i8>,
	// possibly unused
	#[serde(rename = "@RT")]
	pub room_type: Option<String>,
	#[serde(rename = "@SMSR")]
	pub shield_mission: Option<ShieldMission>,
	#[serde(rename = "@WO")]
	// war order and rounds
	pub war_order: Option<WarOrder>,
	#[serde(skip)]
	pub active_player: Option<SGamePlayer>,
}

#[derive(Clone, Debug)]
pub(crate) struct GamePlayerChannels {
	pub xml_channel: XmlPlayerChannel,
	pub command_channel: ServerCommandChannel,
}

impl TriviadorState {
	pub(crate) async fn modify_scores(
		game: SharedTrivGame,
		by: Vec<i16>,
	) -> Result<(), anyhow::Error> {
		let scores = game.read().await.state.players_points.clone();
		let mut scores: Vec<i16> = scores
			.split(',')
			.map(|x| x.parse::<i16>().unwrap())
			.collect();
		for (i, score) in scores.iter_mut().enumerate() {
			*score += by[i];
		}
		game.write().await.state.players_points = format!("{},{},{}", scores[0], scores[1], scores[2]);
		Ok(())
	}

	pub(crate) async fn modify_player_score(
		game: SharedTrivGame,
		rel_id: u8,
		by: i16,
	) -> Result<(), anyhow::Error> {
		let scores = game.read().await.state.players_points.clone();
		let mut scores: Vec<i16> = scores
			.split(',')
			.map(|x| x.parse::<i16>().unwrap())
			.collect();
		for i in 0..3 {
			if i == rel_id {
				scores[i as usize] += by;
				break;
			}
		}
		game.write().await.state.players_points = format!("{},{},{}", scores[0], scores[1], scores[2]);
		Ok(())
	}
}
