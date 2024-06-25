pub struct Building {
	pub level: String,
	pub btype: String,
	pub count: String,
}

pub fn castle(level: u8) -> Building {
	Building {
		level: level.to_string(),
		btype: "0".to_string(),
		count: "0".to_string(),
	}
}

pub fn blacksmith(level: u8) -> Building {
	Building {
		level: level.to_string(),
		btype: "1".to_string(),
		count: "0".to_string(),
	}
}

