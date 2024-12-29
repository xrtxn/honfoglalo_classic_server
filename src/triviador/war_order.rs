use rand::thread_rng;
use rand::prelude::SliceRandom;
use serde::{Serialize, Serializer};
use tracing::error;

#[derive(Debug, Clone)]
pub struct WarOrder {
	pub order: Vec<u8>,
}

// todo account for players getting a base
impl WarOrder {
	pub const NORMAL_ROUND_COUNT: u8 = 6;
	pub(crate) fn new_random_with_size(mut round_count: u8) -> WarOrder {
		if round_count > 6 {
			error!("Round count can't be more than 6, consider splitting it to multiple states");
			round_count = 6;
		}
		let mut order = Vec::with_capacity(round_count as usize * 3);
		for _ in 0..round_count {
			let mut vec: Vec<u8> = vec![1, 2, 3];
			vec.shuffle(&mut thread_rng());
			order.append(&mut vec);
		}
		WarOrder { order }
	}

	fn serialize(&self) -> String {
		let mut serialized = "".to_string();
		for rel_id in self.order.iter() {
			serialized.push_str(rel_id.to_string().as_str());
		}
		serialized
	}

	pub(crate) fn get_next_players(
		&self,
		start: usize,
		amount: usize,
	) -> Result<Vec<u8>, anyhow::Error> {
		let players = self.order[start..(start + amount)].to_vec();
		Ok(players)
	}

	pub(crate) fn get_player_from_order(&self, round: u8, lpnum: u8) -> Result<u8, anyhow::Error> {
		let index = ((round - 1) * 3 + (lpnum - 1)) as usize;
		if index < self.order.len() {
			Ok(self.order[index])
		} else {
			Err(anyhow::anyhow!("Index out of bounds"))
		}
	}
}

impl Serialize for WarOrder {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&Self::serialize(&self))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_war_order() {
		// 123321213231231123
		let wo = WarOrder {
			order: vec![1, 2, 3, 3, 2, 1],
		};

		assert_eq!(wo.get_player_from_order(1, 1).unwrap(), 1);
		assert_eq!(wo.get_player_from_order(2, 1).unwrap(), 3);
		assert_eq!(wo.get_player_from_order(2, 2).unwrap(), 2);
	}

	#[test]
	fn test_get_next_players() {
		let wo = WarOrder {
			order: vec![1, 2, 3, 3, 2, 1],
		};

		assert_eq!(wo.get_next_players(0, 3).unwrap(), vec![1, 2, 3]);
		assert_eq!(wo.get_next_players(3, 3).unwrap(), vec![3, 2, 1]);
	}
}
