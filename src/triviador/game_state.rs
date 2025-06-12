use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub(crate) struct GameState {
	pub state: u8,
	pub round: u8,
	pub phase: u8,
}

impl GameState {
	pub(crate)  fn loading_screen() -> GameState {
		GameState {
			state: 11,
			round: 0,
			phase: 0,
		}
	}
}

impl Serialize for GameState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{},{}", self.state, self.round, self.phase);

		serializer.serialize_str(&s)
	}
}
