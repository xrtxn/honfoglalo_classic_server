use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use serde::{Serialize, Serializer};
use tracing::error;

use super::game_player_data::PlayerName;

// todo make enum struct
#[derive(Debug, Clone)]
pub struct WarOrder {
	pub order: Vec<PlayerName>,
}

// todo account for players getting a base
impl WarOrder {
	pub(crate) const NORMAL_ROUND_COUNT: u8 = 6;
	pub(crate) fn new_random_with_size(mut round_count: u8) -> WarOrder {
		let mut rng = StdRng::from_entropy();
		if round_count > 6 {
			error!("Round count can't be more than 6, consider splitting it to multiple states");
			round_count = 6;
		}
		let mut order = Vec::with_capacity(round_count as usize * 3);
		for _ in 1..=round_count {
			let mut vec: Vec<PlayerName> = vec![
				PlayerName::Player1,
				PlayerName::Player2,
				PlayerName::Player3,
			];
			vec.shuffle(&mut rng);
			order.append(&mut vec);
		}
		WarOrder { order }
	}

	pub(crate) fn standard_round() -> WarOrder {
		// todo make the last round calculated
		// the first should be the point leader, second is the second player, and the last is the last player
		let mut order = Vec::with_capacity(18);
		for round in 0..6 {
			// this shifts around players starting from the first player in each round
			let start = round % 3;
			order.push(PlayerName::from((start + 1) as u8));
			order.push(PlayerName::from(((start + 1) % 3 + 1) as u8));
			order.push(PlayerName::from(((start + 2) % 3 + 1) as u8));
		}
		WarOrder::from(order)
	}

	fn serialize(&self) -> String {
		let mut serialized = "".to_string();
		for rel_id in self.order.iter() {
			serialized.push_str((*rel_id as u8).to_string().as_str());
		}
		serialized
	}

	pub(crate) fn get_next_players(
		&self,
		start: usize,
		amount: usize,
	) -> Result<Vec<PlayerName>, anyhow::Error> {
		let players = self.order[start..(start + amount)].to_vec();
		Ok(players)
	}
}

impl From<Vec<PlayerName>> for WarOrder {
	fn from(counties: Vec<PlayerName>) -> Self {
		WarOrder { order: counties }
	}
}

impl From<Vec<u8>> for WarOrder {
	fn from(counties: Vec<u8>) -> Self {
		let order: Vec<PlayerName> = counties.iter().map(|x| PlayerName::from(*x)).collect();
		WarOrder { order }
	}
}

impl Serialize for WarOrder {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&Self::serialize(self))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_next_players() {
		let wo = WarOrder::from(vec![1, 2, 3, 3, 2, 1]);

		assert_eq!(
			wo.get_next_players(0, 3).unwrap(),
			vec![
				PlayerName::Player1,
				PlayerName::Player2,
				PlayerName::Player3
			]
		);
		assert_eq!(
			wo.get_next_players(3, 3).unwrap(),
			vec![
				PlayerName::Player3,
				PlayerName::Player2,
				PlayerName::Player1
			]
		);
	}
}
