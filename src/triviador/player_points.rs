use std::collections::HashMap;

use serde::{Serialize, Serializer};

#[derive(Clone, Debug)]
pub(crate) struct PlayerPoints(HashMap<u8, i16>);

impl PlayerPoints {
	pub(crate) fn new() -> PlayerPoints {
		let mut points = HashMap::new();
		for i in 1..=3 {
			points.insert(i, 0);
		}
		Self(points)
	}

	pub(crate) fn set_player_points(&mut self, rel_id: &u8, points: i16) {
		let old_points = self.0.get_mut(rel_id).unwrap();
		*old_points = points;
	}

	pub(crate) fn change_player_points(&mut self, rel_id: &u8, by: i16) {
		let old_points = self.0.get_mut(rel_id).unwrap();
		*old_points += by;
	}
}

impl Serialize for PlayerPoints {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!(
			"{},{},{}",
			self.0.get(&1).unwrap(),
			self.0.get(&2).unwrap(),
			self.0.get(&3).unwrap()
		);

		serializer.serialize_str(&s)
	}
}
