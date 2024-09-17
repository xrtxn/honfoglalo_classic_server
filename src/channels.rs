use serde::{Deserialize, Serialize};

use crate::channels::command::request::CommandRequest;
use crate::channels::command::response::CommandResponseHeader;
use crate::channels::listen::request::ListenRequest;
use crate::channels::listen::response::ListenResponseHeader;

pub mod command;
pub mod listen;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ERROR")]
pub struct ErrorResponse {}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "CH")]
pub enum ChannelType {
	#[serde(rename = "C")]
	Command(CommandRequest),
	#[serde(rename = "L")]
	Listen(ListenRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseHeaders {
	#[serde(rename = "C")]
	Command(CommandResponseHeader),
	#[serde(rename = "L")]
	Listen(ListenResponseHeader),
}
