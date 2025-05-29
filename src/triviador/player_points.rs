use std::collections::HashMap;

use serde::{Serialize, Serializer};

use super::game_player_data::PlayerName;

#[derive(Clone, Debug)]
pub(crate) struct PlayerPoints(HashMap<PlayerName, i16>);

impl PlayerPoints {
	pub(crate) fn new() -> PlayerPoints {
		let mut points = HashMap::new();
		points.insert(PlayerName::Player1, 0);
		points.insert(PlayerName::Player2, 0);
		points.insert(PlayerName::Player3, 0);
		Self(points)
	}

	pub(crate) fn set_player_points(&mut self, rel_id: &PlayerName, points: i16) {
		let old_points = self.0.get_mut(rel_id).unwrap();
		*old_points = points;
	}

	pub(crate) fn change_player_points(&mut self, rel_id: &PlayerName, by: i16) {
		let old_points = self.0.get_mut(rel_id).unwrap();
		*old_points += by;
	}

	pub(crate) fn get_player_points(&self, rel_id: &PlayerName) -> i16 {
        *self.0.get(rel_id).unwrap()
    }
}

impl Serialize for PlayerPoints {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!(
			"{},{},{}",
			self.0.get(&PlayerName::Player1).unwrap(),
			self.0.get(&PlayerName::Player2).unwrap(),
			self.0.get(&PlayerName::Player3).unwrap()
		);

		serializer.serialize_str(&s)
	}
}
