
use fred::prelude::RedisPool;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use tracing::error;

use crate::triviador::triviador_state::TriviadorState;

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

	pub(crate) async fn set_redis(
		&self,
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<u8, anyhow::Error> {
		TriviadorState::set_field(temp_pool, game_id, "war_order", self.serialize().as_str()).await
	}

	pub(crate) async fn get_redis(
		temp_pool: &RedisPool,
		game_id: u32,
	) -> Result<WarOrder, anyhow::Error> {
		let order = TriviadorState::get_field(temp_pool, game_id, "war_order").await?;
		let mut order_vec = Vec::with_capacity(order.len());
		for c in order.chars() {
			order_vec.push(c.to_digit(10).unwrap() as u8);
		}
		Ok(WarOrder { order: order_vec })
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
