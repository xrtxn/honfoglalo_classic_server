pub mod request {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug)]
	pub struct ListenRequest {
		#[serde(rename = "CID")]
		pub client_id: String,
		#[serde(rename = "MN")]
		pub mn: String,
		#[serde(rename = "TRY")]
		pub retry_num: String,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct ListenXML {
		#[serde(rename = "@READY")]
		pub is_ready: u8,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct ListenRoot {
		// #[serde(rename = "$value")]
		// pub header_type: Headers,
		#[serde(rename = "$value")]
		pub listen_type: ListenType,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub enum ListenType {
		#[serde(rename = "LISTEN")]
		Listen(ListenXML),
	}
}

pub mod response {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "L")]
	pub struct ListenResponse {
		#[serde(rename = "@CID")]
		pub client_id: String,
		#[serde(rename = "@MN")]
		pub mn: String,
		#[serde(rename = "@R")]
		pub result: u8,
	}
}
