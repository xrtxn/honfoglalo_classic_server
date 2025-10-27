use std::fmt::{self, Display};
use std::str::FromStr;

use anyhow::bail;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize, Serializer};

use crate::emulator::Emulator;

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum PlayerName {
	Nobody = 0,
	Player1 = 1,
	Player2 = 2,
	Player3 = 3,
}

impl PlayerName {
	pub(crate) fn all() -> impl Iterator<Item = PlayerName> {
		[
			PlayerName::Player1,
			PlayerName::Player2,
			PlayerName::Player3,
		]
		.iter()
		.copied()
	}
}

impl From<u8> for PlayerName {
	fn from(val: u8) -> Self {
		match val {
			0 => PlayerName::Nobody,
			1 => PlayerName::Player1,
			2 => PlayerName::Player2,
			3 => PlayerName::Player3,
			_ => panic!("Invalid PlayerName value: {}", val),
		}
	}
}
impl Display for PlayerName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let val = *self as u8;
		write!(f, "{}", val)
	}
}
impl Serialize for PlayerName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let val = *self as u8;
		serializer.serialize_u8(val)
	}
}

#[derive(Debug, Clone)]
pub struct GamePlayerData {
	pub id: i32,
	pub xp_points: i32,
	pub xp_level: i16,
	pub game_count: i32,
	// meaning?
	pub game_count_sr: i32,
	pub country_id: String,
	pub castle_level: i16,
	// this can be not existent with `,`
	pub custom_avatar: bool,
	pub soldier: i16,
	pub act_league: i16,
}

impl GamePlayerData {
	pub(crate) fn new_player(id: i32) -> GamePlayerData {
		let mut rng = StdRng::from_entropy();
		// there could be more types
		let soldier = rng.gen_range(1..8);

		GamePlayerData {
			id,
			xp_points: 14000,
			xp_level: 15,
			game_count: 1,
			game_count_sr: 0,
			country_id: "hu".to_string(),
			castle_level: 1,
			custom_avatar: false,
			soldier,
			act_league: 1,
		}
	}

	pub fn new_bot() -> GamePlayerData {
		let mut rng = StdRng::from_entropy();
		// there could be more types
		let soldier = rng.gen_range(1..8);

		GamePlayerData {
			id: -1,
			xp_points: 14000,
			xp_level: 15,
			game_count: 1,
			game_count_sr: 0,
			country_id: "hu".to_string(),
			castle_level: 1,
			custom_avatar: false,
			soldier,
			act_league: 1,
		}
	}

	pub(crate) fn is_bot(&self) -> bool {
		self.id == -1
	}
}

// Emulates a Player
impl Emulator for GamePlayerData {
	fn emulate() -> Self {
		let mut rng = StdRng::from_entropy();
		let xp_points = rng.gen_range(100..100000);
		let xp_level = rng.gen_range(1..100);
		// there could be more types
		let soldier = rng.gen_range(1..8);

		GamePlayerData {
			id: 1,
			xp_points,
			xp_level,
			game_count: 1,
			game_count_sr: 0,
			country_id: "hu".to_string(),
			castle_level: 1,
			custom_avatar: false,
			soldier,
			act_league: 1,
		}
	}
}

impl FromStr for GamePlayerData {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parts: Vec<&str> = s.split(',').collect();

		let id = parts[0].parse::<i32>()?;
		let xp_points = parts[1].parse::<i32>()?;
		let xp_level = parts[2].parse::<i16>()?;
		let game_count = parts[3].parse::<i32>()?;
		let game_count_sr = parts[4].parse::<i32>()?;
		let country_id = parts[5].to_string();
		let castle_level = parts[6].parse::<i16>()?;

		// Handle custom_avatar as an optional boolean
		let custom_avatar = match parts[7] {
			"" => false, // Empty string represents a false value
			"true" => true,
			"false" => false,
			_ => bail!("Invalid custom_avatar value"),
		};

		let soldier = parts[8].parse::<i16>()?;
		let act_league = parts[9].parse::<i16>()?;

		Ok(GamePlayerData {
			id,
			xp_points,
			xp_level,
			game_count,
			game_count_sr,
			country_id,
			castle_level,
			custom_avatar,
			soldier,
			act_league,
		})
	}
}

impl fmt::Display for GamePlayerData {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let avatar = if self.custom_avatar {
			"todo_this_is_a_custom_avatar"
		} else {
			""
		};

		let str = format!(
			"{},{},{},{},{},{},{},{},{},{}",
			self.id,
			self.xp_points,
			self.xp_level,
			self.game_count,
			self.game_count_sr,
			self.country_id,
			self.castle_level,
			avatar,
			self.soldier,
			self.act_league
		);
		write!(f, "{}", str)
	}
}

impl Serialize for GamePlayerData {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.to_string().as_str())
	}
}
