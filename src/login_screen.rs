use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginXML {
	#[serde(rename = "@UID")]
	pub uid: String,
	#[serde(rename = "@NAME")]
	pub name: String,
	#[serde(rename = "@KV")]
	pub kv: String,
	#[serde(rename = "@KD")]
	pub kd: String,
	#[serde(rename = "@LOGINSYSTEM")]
	pub loginsystem: String,
	#[serde(rename = "@CLIENTTYPE")]
	pub clienttype: String,
	#[serde(rename = "@CLIENTVER")]
	pub clientver: String,
	#[serde(rename = "@PAGEMODE")]
	pub pagemode: String,
	#[serde(rename = "@EXTID")]
	pub extid: String,
	#[serde(rename = "@TIME")]
	pub time: String,
	#[serde(rename = "@GUID")]
	pub guid: String,
	#[serde(rename = "@SIGN")]
	pub sign: String,
	#[serde(rename = "@PSID")]
	pub psid: String,
	#[serde(rename = "@CC")]
	pub cc: String,
	#[serde(rename = "@WH")]
	pub wh: String,
}
