use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::Read;
use std::str::FromStr;

use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use scc::HashMap;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::channels::command::request::CommandRequest;
use crate::channels::command::response::CommandResponseHeader;
use crate::channels::listen::request::ListenRequest;
use crate::channels::listen::response::ListenResponseHeader;

pub mod command;
pub mod listen;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ERROR")]
pub struct ChannelErrorResponse {}
impl ChannelErrorResponse {
	pub fn new() -> ChannelErrorResponse {
		ChannelErrorResponse {}
	}
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "CH")]
pub enum QueryChannelType {
	#[serde(rename = "C")]
	Command(CommandRequest),
	#[serde(rename = "L")]
	Listen(ListenRequest),
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub enum BodyChannelType {
	#[serde(rename = "C")]
	Command(CommandRequest),
	#[serde(rename = "L")]
	Listen(ListenRequest),
}

#[derive(Deserialize)]
struct CommandRequestHelper {
	#[serde(rename = "CID")]
	client_id: i32,
	#[serde(rename = "MN")]
	mn: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseHeaders {
	#[serde(rename = "C")]
	Command(CommandResponseHeader),
	#[serde(rename = "L")]
	Listen(ListenResponseHeader),
}

pub fn parse_xml_multiple(xml: &str) -> Result<BodyChannelType, anyhow::Error> {
	let mut reader = Reader::from_str(xml);
	reader.config_mut().trim_text(true);

	let mut cid = 0;
	let mut mn = 0;
	let mut buf = Vec::new();

	// The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
	loop {
		let event = reader.read_event_into(&mut buf);
		// NOTE: this is the generic case when we don't know about the input BufRead.
		// when the input is a &str or a &[u8], we don't actually need to use another
		// buffer, we could directly call `reader.read_event()`
		match event {
			Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
			// exits the loop when reaching end of file
			Ok(Event::Eof) => {
				break;
			}

			Ok(Event::Empty(e)) => match e.name().as_ref() {
				b"C" => {
					for attr in e.attributes() {
						let attr = attr.unwrap();
						match attr.key.as_ref() {
							b"CID" => {
								let value = std::str::from_utf8(&attr.value).unwrap();
								cid = value.parse::<i32>().unwrap_or_else(|_| 0);
							}
							b"MN" => {
								let value = std::str::from_utf8(&attr.value).unwrap();
								mn = value.parse::<u32>().unwrap();
							}
							_ => println!("Other attribute"),
						}
					}
					return Ok(BodyChannelType::Command(CommandRequest {
						client_id: cid,
						mn,
						retry_num: None,
					}));
				}
				b"L" => {
					for attr in e.attributes() {
						let attr = attr.unwrap();
						match attr.key.as_ref() {
							b"CID" => {
								let value = std::str::from_utf8(&attr.value).unwrap();
								cid = value.parse::<i32>().unwrap();
							}
							b"MN" => {
								let value = std::str::from_utf8(&attr.value).unwrap();
								mn = value.parse::<u32>().unwrap();
							}
							_ => println!("Other attribute"),
						}
					}
					return Ok(BodyChannelType::Listen(ListenRequest {
						client_id: cid,
						mn,
						retry_num: None,
					}));
				}
				_ => println!("Other character"),
			},

			// There are several other `Event`s we do not consider here
			_ => println!("Other event"),
		}
		buf.clear();
	}
	Ok(BodyChannelType::Command(CommandRequest {
		client_id: cid,
		mn,
		retry_num: None,
	}))
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_ser_tokens, Token};

	use super::*;

	#[test]
	fn test_deserialize_body_channel_type() {
		let xml = r#"<C CID="1" MN="1" />"#;
		// let body_channel_type = parse_xml_multiple(xml);
		// let body_channel_type: BodyChannelType = quick_xml::de::from_str(xml).unwrap();
		let direct = parse_xml_multiple(xml);
		assert_eq!(
			direct.unwrap(),
			BodyChannelType::Command(CommandRequest {
				client_id: 1,
				mn: 1,
				retry_num: None
			})
		);

		let xml = r#"<L CID="32" MN="64" />"#;
		let direct = parse_xml_multiple(xml);
		assert_eq!(
			direct.unwrap(),
			BodyChannelType::Listen(ListenRequest {
				client_id: 32,
				mn: 64,
				retry_num: None
			})
		);
	}
}
