use serde::Serialize;
use serde_with::skip_serializing_none;

use super::areas::Areas;
use super::player_points::PlayerPoints;
use super::war_order::WarOrder;
use crate::app::ServerCommandChannel;
use crate::app::XmlPlayerChannel;
use crate::game_handlers::s_game::SGamePlayer;
use crate::triviador::available_area::AvailableAreas;
use crate::triviador::bases::Bases;
use crate::triviador::game_state::GameState;
use crate::triviador::round_info::RoundInfo;
use crate::triviador::selection::Selection;
use crate::triviador::shield_mission::ShieldMission;

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
	pub players_points: PlayerPoints,
	#[serde(rename = "@SEL")]
	pub selection: Selection,
	#[serde(rename = "@B")]
	pub base_info: Bases,
	#[serde(rename = "@A", serialize_with = "Areas::areas_serializer")]
	// todo replace this with an enum struct
	pub areas_info: Areas,
	#[serde(rename = "@AA", serialize_with = "AvailableAreas::available_serialize")]
	pub available_areas: AvailableAreas,
	#[serde(rename = "@UH")]
	pub used_helps: String,
	#[serde(rename = "@FAO")]
	pub fill_round_winners: String,
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
