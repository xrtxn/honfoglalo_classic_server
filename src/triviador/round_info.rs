use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct RoundInfo {
	// lpnum
	pub mini_phase_num: u8,
	pub rel_player_id: u8,
	pub attacked_player: Option<u8>,
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = match self.attacked_player {
			Some(attacked) => format!(
				"{},{},{}",
				self.mini_phase_num, self.rel_player_id, attacked
			),
			None => format!("{},{}", self.mini_phase_num, self.rel_player_id),
		};

		serializer.serialize_str(&s)
	}
}
