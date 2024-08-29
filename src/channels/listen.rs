pub mod request {
	use serde::{Deserialize, Serialize};
	use serde_aux::prelude::deserialize_bool_from_anything;

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
		#[serde(rename = "@READY", deserialize_with = "deserialize_bool_from_anything")]
		pub is_ready: bool,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct ListenRoot {
		// #[serde(rename = "$value")]
		// pub header_type: Headers,
		#[serde(rename = "LISTEN")]
		pub listen: ListenXML,
	}
}

pub mod response {
	use serde::{Deserialize, Serialize};

	use crate::village::setup::VillageSetupRoot;
	use crate::village::start::friendly_game::ActiveSepRoom;

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "ROOT")]
	pub struct ListenResponse {
		#[serde(rename = "L")]
		header: ListenResponseHeader,
		// todo - this sucks, but there is no better option for now, because xml serializing sucks
		#[serde(rename = "ROOT")]
		message: ListenResponseType,
	}

	impl ListenResponse {
		pub fn new(header: ListenResponseHeader, message: ListenResponseType) -> ListenResponse {
			ListenResponse { header, message }
		}
	}

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "L")]
	pub struct ListenResponseHeader {
		#[serde(rename = "@CID")]
		pub client_id: String,
		#[serde(rename = "@MN")]
		pub mn: String,
		#[serde(rename = "@R")]
		pub result: u8,
	}

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(untagged)]
	pub enum ListenResponseType {
		VillageSetup(VillageSetupRoot),
		ActiveSepRoom(ActiveSepRoom),
	}
}
