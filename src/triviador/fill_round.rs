use std::collections::VecDeque;

use serde::{Serialize, Serializer};

use super::game_player_data::PlayerName;

#[derive(Debug, Clone)]
pub(crate) struct FillRound {
	players: VecDeque<PlayerName>,
}

impl FillRound {
	pub(crate) fn new() -> Self {
		FillRound {
			players: VecDeque::new(),
		}
	}

	pub(crate) fn add_player(&mut self, player: Option<PlayerName>) {
		if let Some(player) = player {
			self.players.push_back(player);
		}
	}
}

impl Serialize for FillRound {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		use serde::ser::SerializeSeq;

		let mut seq = serializer.serialize_seq(Some(self.players.len()))?;
		for player in &self.players {
			seq.serialize_element(player)?;
		}
		seq.end()
	}
}
