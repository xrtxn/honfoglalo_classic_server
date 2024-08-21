pub mod info_help {
	use serde::{Deserialize, Serialize};

	use crate::emulator::HungaryEmulator;

	#[derive(Serialize, Deserialize, Debug)]
	pub struct HelpResponse {
		pub error: String,
		pub data: HelpData,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct HelpData {
		pub todo: Inner,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct Inner {
		#[serde(rename = "tf")]
		pub text_field: String,
		#[serde(rename = "hid")]
		pub help_id: String,
	}

	impl HungaryEmulator for HelpResponse {
		fn emulate(_: String) -> HelpResponse {
			HelpResponse {
				error: "0".to_string(),
				data: HelpData {
					todo: Inner {
						text_field: "this is the text".to_string(),
						help_id: "0".to_string(),
					},
				},
			}
		}
	}

}
