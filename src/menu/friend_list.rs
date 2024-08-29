pub mod friends {
	use serde::{Deserialize, Serialize};

	use crate::emulator::Emulator;

	#[derive(Serialize, Deserialize, Debug)]
	pub struct FriendResponse {
		pub error: String,
		pub data: Friends,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct FriendDetails {
		pub id: String,
		pub name: String,
		pub int_avatar: String,
		pub flag: String,
		pub actleague: String,
		pub xplevel: String,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct Friends {
		pub allitems: Vec<FriendDetails>,
	}

	impl Emulator for FriendResponse {
		fn emulate() -> FriendResponse {
			FriendResponse {
				error: "0".to_string(),
				data: Friends {
					allitems: vec![FriendDetails {
						id: "2".to_string(),
						name: "Lajos".to_string(),
						int_avatar: "0".to_string(),
						flag: "0".to_string(),
						actleague: "1".to_string(),
						xplevel: "1".to_string(),
					}],
				},
			}
		}
	}
}

pub mod external_data {
	use serde::{Deserialize, Serialize};

	use crate::emulator::Emulator;

	#[derive(Serialize, Deserialize, Debug)]
	pub struct ExtDataRequest {
		#[serde(rename = "@IDLIST")]
		pub requested_ids: String,
	}

	#[derive(Serialize, Deserialize, Debug)]
	#[serde(rename = "ROOT")]
	pub struct ExternalFriendsRoot {
		#[serde(rename = "EXTDATA")]
		pub extdata: Extdata,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct Extdata {
		#[serde(rename = "USER")]
		pub user: Vec<User>,
	}

	#[derive(Serialize, Deserialize, Debug)]
	pub struct User {
		#[serde(rename = "ID")]
		pub id: String,
		#[serde(rename = "NAME")]
		pub name: String,
		#[serde(rename = "USECUSTOM")]
		pub usecustom: String,
		#[serde(rename = "CUSTOM")]
		pub custom: String,
		#[serde(rename = "IMGURL")]
		pub imgurl: String,
		#[serde(rename = "ONLINE")]
		pub online: String,
	}

	impl Emulator for ExternalFriendsRoot {
		fn emulate() -> Self {
			ExternalFriendsRoot {
				extdata: Extdata {
					user: vec![
						User {
							id: "2".to_string(),
							name: "foo".to_string(),
							usecustom: "0".to_string(),
							custom: "todo".to_string(),
							imgurl: "//graph.facebook.com/1/picture".to_string(),
							online: "1".to_string(),
						},
						User {
							id: "3".to_string(),
							name: "bar".to_string(),
							usecustom: "0".to_string(),
							custom: "todo".to_string(),
							imgurl: "//graph.facebook.com/1/picture".to_string(),
							online: "0".to_string(),
						},
					],
				},
			}
		}
	}
}
