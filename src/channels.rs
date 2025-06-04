use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};

use crate::channels::command::request::CommandRequest;
use crate::channels::command::response::CommandResponseHeader;
use crate::channels::heartbeat::request::HeartBeatRequest;
use crate::channels::listen::request::ListenRequest;
use crate::channels::listen::response::ListenResponseHeader;

pub mod command;
pub mod heartbeat;
pub mod listen;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ERROR")]
pub struct ChannelErrorResponse {}
impl ChannelErrorResponse {
	pub fn new() -> ChannelErrorResponse {
		ChannelErrorResponse {}
	}
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub enum BodyChannelType {
	#[serde(rename = "C")]
	Command(CommandRequest),
	#[serde(rename = "L")]
	Listen(ListenRequest),
	#[serde(rename = "H")]
	HeartBeat(HeartBeatRequest),
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

	loop {
		let event = reader.read_event_into(&mut buf);
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
								cid = value.parse::<i32>().unwrap_or(0);
							}
							b"MN" => {
								let value = std::str::from_utf8(&attr.value).unwrap();
								mn = value.parse::<u32>().unwrap();
							}
							_ => println!("Unknown attribute for C"),
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
							_ => println!("Unknown attribute for L"),
						}
					}
					return Ok(BodyChannelType::Listen(ListenRequest {
						client_id: cid,
						mn,
						retry_num: None,
					}));
				}
				b"H" => {
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
							//optional attribute
							b"MLP" => {
								// we don't care about this for now
							}
							_ => println!("Unknown attribute for H"),
						}
					}
					return Ok(BodyChannelType::HeartBeat(HeartBeatRequest {
						client_id: cid,
						mn,
					}));
				}
				_ => println!("Unknown character"),
			},

			_ => println!("Unknown event"),
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
	use super::*;

	#[test]
	fn test_deserialize_body_channel_type() {
		let xml = r#"<C CID="1" MN="1" />"#;
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
