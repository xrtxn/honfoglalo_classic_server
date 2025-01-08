use serde::{Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct ShieldMission {
	pub shieldmission: i32,
	pub shieldmission_rt: i32,
}

impl Serialize for ShieldMission {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// hexadecimal
		let s = format!("{:X},{:X}", self.shieldmission, self.shieldmission_rt);
		serializer.serialize_str(&s)
	}
}
