use serde::{Deserialize, Serialize};

// todo better structure
#[derive(Serialize, Deserialize, Debug)]
pub struct GetExternalData {
	#[serde(rename = "@IDLIST")]
	id_list: String,
}
