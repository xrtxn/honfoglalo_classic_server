pub mod request {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
	pub struct HeartBeatRequest {
		#[serde(rename = "CID")]
		pub client_id: i32,
		#[serde(rename = "MN")]
		pub mn: u32,
	}

	pub mod response {
		use serde::{Deserialize, Serialize};
		use serde_with::skip_serializing_none;

		#[skip_serializing_none]
		#[derive(Serialize, Deserialize, Debug)]
		#[serde(rename = "ROOT")]
		pub struct HeartBeatResponse {
			#[serde(rename = "H")]
			header: HeartBeatResponseHeader,
		}

		impl HeartBeatResponse {
			pub fn ok(cid: impl ToString, mn: impl ToString) -> HeartBeatResponse {
				HeartBeatResponse {
					header: HeartBeatResponseHeader {
						client_id: cid.to_string(),
						mn: mn.to_string(),
						result: 0,
						status: 1,
					},
				}
			}

			#[allow(dead_code)]
			pub fn timeout(cid: impl ToString, mn: impl ToString) -> HeartBeatResponse {
				HeartBeatResponse {
					header: HeartBeatResponseHeader {
						client_id: cid.to_string(),
						mn: mn.to_string(),
						result: 0,
						status: 2,
					},
				}
			}

			#[allow(dead_code)]
			pub fn reconnect(cid: impl ToString, mn: impl ToString) -> HeartBeatResponse {
				HeartBeatResponse {
					header: HeartBeatResponseHeader {
						client_id: cid.to_string(),
						mn: mn.to_string(),
						result: 3,
						status: 0,
					},
				}
			}

			#[allow(dead_code)]
			pub fn error(cid: impl ToString, mn: impl ToString) -> HeartBeatResponse {
				HeartBeatResponse {
					header: HeartBeatResponseHeader {
						client_id: cid.to_string(),
						mn: mn.to_string(),
						result: 1,
						status: 0,
					},
				}
			}
		}

		#[derive(Serialize, Deserialize, Debug)]
		#[serde(rename = "H")]
		pub struct HeartBeatResponseHeader {
			#[serde(rename = "@CID")]
			pub client_id: String,
			#[serde(rename = "@MN")]
			pub mn: String,
			#[serde(rename = "@R")]
			pub result: u8,
			/// if even indicates a timeout
			#[serde(rename = "@S")]
			pub status: u8,
		}
	}
}
