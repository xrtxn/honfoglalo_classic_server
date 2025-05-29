use serde::{Serialize, Serializer};

use super::game_player_data::PlayerName;

#[derive(Debug, Clone)]
pub struct RoundInfo {
	// lpnum
	/// The little arrow under the war order
	pub mini_phase_num: u8,
	pub active_player: PlayerName,
	pub attacked_player: Option<PlayerName>,
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = match self.attacked_player {
			Some(attacked) => format!(
				"{},{},{}",
				self.mini_phase_num, self.active_player, attacked
			),
			None => format!("{},{}", self.mini_phase_num, self.active_player),
		};

		serializer.serialize_str(&s)
	}
}
