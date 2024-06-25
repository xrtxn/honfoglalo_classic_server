use serde::{Deserialize, Serialize};
use serde_aux::prelude::{deserialize_number_from_string, deserialize_string_from_number};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd")]
pub enum Mobile {
	#[serde(rename = "ping")]
	Ping(PingRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingRequest {}

