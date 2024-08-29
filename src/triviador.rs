use std::collections::HashSet;

use serde::ser::{SerializeStruct, SerializeTuple};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::skip_serializing_none;

use crate::triviador::county::*;

#[derive(Debug)]
struct PlayerData {
	points: u16,
	// to string
	chat_state: u8,
	// convert to number
	is_connected: bool,
	// triviador.Game line 1748
	base: Base,
	// the number of area in hexadecimal
	selection: u8,
}

#[derive(PartialEq, Debug)]
struct Base {
	base_id: u8,
	towers_destroyed: u8,
}

impl Base {
	pub fn new_tower_destroyed(&mut self) {
		self.towers_destroyed += 1;
	}
}

impl Serialize for Base {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let base_part = self.towers_destroyed << 6;
		let hex = format!("{:02x}", self.base_id + base_part);
		serializer.serialize_str(&hex)
	}
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum PlayerNames {
	Player1,
	Player2,
	Player3,
}

#[derive(Clone, Debug)]
struct Area {
	owner: u8,
	is_fortress: bool,
	value: AreaValue,
}

impl Serialize for Area {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut ac = self.owner;
		let vc = (self.value.clone() as u8) << 4;
		ac += vc;

		if self.is_fortress {
			ac |= 128;
		}

		let hex = format!("{:02x}", ac);
		serializer.serialize_str(&hex)
	}
}

#[derive(Serialize, Clone, Debug)]
// todo use the names instead of values
enum AreaValue {
	_1000 = 1,
	_400 = 2,
	_300 = 3,
	_200 = 4,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_ser_tokens, Token};

	use super::*;
	#[test]
	fn base_test() {
		let base = &Base {
			base_id: 2,
			towers_destroyed: 2,
		};
		assert_ser_tokens(&base, &[Token::String("82")]);

		let base = &Base {
			base_id: 8,
			towers_destroyed: 0,
		};
		assert_ser_tokens(&base, &[Token::String("08")]);
	}

	#[test]
	fn area_test() {
		let area = Area {
			owner: 1,
			is_fortress: false,
			value: AreaValue::_200,
		};

		assert_ser_tokens(&area, &[Token::String("41")]);
		let area = Area {
			owner: 3,
			is_fortress: false,
			value: AreaValue::_1000,
		};

		assert_ser_tokens(&area, &[Token::String("13")]);
	}

	#[test]
	fn county_serialize() {
		let decoded = decode_available_areas(i32::from_str_radix("07FFFF", 16).unwrap());
		assert_eq!(
			decoded,
			vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
		);
		assert_eq!(
			encode_available_areas(vec![
				1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18, 19
			],),
			"077FFF"
		)
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug)]
#[serde(rename = "ROOT")]
pub struct TriviadorResponseRoot {
	#[serde(rename = "STATE")]
	pub state: TriviadorState,
	#[serde(rename = "PLAYERS")]
	pub players: Info,
	#[serde(rename = "CMD")]
	pub cmd: Option<Cmd>,
}

impl TriviadorResponseRoot {
	pub fn new_game() -> TriviadorResponseRoot {
		TriviadorResponseRoot {
			state: TriviadorState {
				map_name: "MAP_WD".to_string(),
				game_state: GameState {
					state: 11,
					gameround: 0,
					phase: 0,
				},
				round_info: RoundInfo {
					lpnum: 0,
					next_player: 0,
				},
				players_connected: "123".to_string(),
				players_chat_state: "0,0,0".to_string(),
				players_points: "0,0,0".to_string(),
				selection: "000000".to_string(),
				base_info: "000000".to_string(),
				area_num: "0000000000000000000000000000000000000000".to_string(),
				available_areas: Some(HashSet::new()),
				used_helps: "0".to_string(),
				room_type: None,
				shield_mission: None,
				war: None,
			},
			players: Info {
				p1: "xrtxn".to_string(),
				p2: "null".to_string(),
				p3: "null".to_string(),
				pd1: "-1,14000,15,1,0,ar,1,,0".to_string(),
				pd2: "-1,14000,15,1,0,hu,1,,8".to_string(),
				pd3: "-1,14000,15,1,0,ar,1,,6".to_string(),
				you: "1,2,3".to_string(),
				gameid: "1".to_string(),
				room: "1".to_string(),
				rules: "0,0".to_string(),
			},
			cmd: None,
		}
	}

	pub fn announcement(&mut self) {
		let triviador = self;
		triviador.state.game_state.state = 1;
	}
	pub fn choose_area(&mut self) {
		let triviador = self;
		triviador.state.game_state.phase = 1;
		triviador.state.round_info = RoundInfo {
			lpnum: 1,
			next_player: 1,
		};
		triviador.state.available_areas = Some(Cmd::all_counties());
		triviador.cmd = Some(Cmd {
			command: "SELECT".to_string(),
			available: Cmd::all_counties(),
			cmd_timeout: 10,
		});
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug)]
pub struct TriviadorState {
	#[serde(rename = "@SCR")]
	pub map_name: String,
	#[serde(rename = "@ST")]
	pub game_state: GameState,
	#[serde(rename = "@CP")]
	pub round_info: RoundInfo,
	#[serde(rename = "@HC")]
	// numbers of players connected e.g. 1,2,3
	pub players_connected: String,
	#[serde(rename = "@CHS")]
	pub players_chat_state: String,
	#[serde(rename = "@PTS")]
	pub players_points: String,
	#[serde(rename = "@SEL")]
	// 2 digits -> 1 player, first 2 digit 1st player...
	// todo
	pub selection: String,
	#[serde(rename = "@B")]
	// todo
	pub base_info: String,
	#[serde(rename = "@A")]
	// used by: Math.floor(Util.StringVal(tag.A).length / 2);
	pub area_num: String,
	#[serde(rename = "@AA", serialize_with = "available_serialize")]
	pub available_areas: Option<HashSet<County>>,
	#[serde(rename = "@UH")]
	pub used_helps: String,
	// possibly unused
	#[serde(rename = "@RT")]
	pub room_type: Option<String>,
	#[serde(rename = "@SMSR")]
	pub shield_mission: Option<ShieldMission>,
	#[serde(rename = "@WO")]
	// war order and rounds
	pub war: Option<String>,
}

impl TriviadorState {
	pub fn bazisterulet_sorsolas(&mut self) {
		self.game_state.state = 1;
	}
}

#[derive(Serialize, Debug)]
pub struct Info {
	#[serde(rename = "@P1")]
	pub p1: String,
	#[serde(rename = "@P2")]
	pub p2: String,
	#[serde(rename = "@P3")]
	pub p3: String,
	#[serde(rename = "@PD1")]
	pub pd1: String,
	#[serde(rename = "@PD2")]
	pub pd2: String,
	#[serde(rename = "@PD3")]
	pub pd3: String,
	#[serde(rename = "@YOU")]
	pub you: String,
	#[serde(rename = "@GAMEID")]
	pub gameid: String,
	#[serde(rename = "@ROOM")]
	pub room: String,
	#[serde(rename = "@RULES")]
	// this gives an interesting twist, 2 area instead of 1
	// possible values 1,0
	pub rules: String,
}

mod county {
	use std::collections::HashSet;

	use super::*;
	#[derive(Debug, Serialize)]
	pub struct Cmd {
		#[serde(rename = "@CMD")]
		pub command: String,
		#[serde(rename = "@AVAILABLE", serialize_with = "county_serialize")]
		pub available: HashSet<County>,
		#[serde(rename = "@TO")]
		// seconds for action
		pub cmd_timeout: u8,
	}

	impl Cmd {
		pub(crate) fn all_counties() -> HashSet<County> {
			HashSet::from([
				County::Pest,
				County::Nograd,
				County::Heves,
				County::JaszNagykunSzolnok,
				County::BacsKiskun,
				County::Fejer,
				County::KomaromEsztergom,
				County::Borsod,
				County::HajduBihar,
				County::Bekes,
				County::Csongrad,
				County::Tolna,
				County::Somogy,
				County::Veszprem,
				County::GyorMosonSopron,
				County::SzabolcsSzatmarBereg,
				County::Baranya,
				County::Zala,
				County::Vas,
			])
		}

		pub(crate) fn remove_county(&mut self, county: County) {
			self.available.remove(&county);
		}
	}

	#[derive(Serialize, Clone, Copy, Eq, PartialEq, Hash, Debug)]
	pub enum County {
		None = 0,                  // if nothing is selected
		Pest = 1,                  // Pest
		Nograd = 2,                // Nógrád
		Heves = 3,                 // Heves
		JaszNagykunSzolnok = 4,    // Jász-Nagykun-Szolnok
		BacsKiskun = 5,            // Bács-Kiskun
		Fejer = 6,                 // Fejér
		KomaromEsztergom = 7,      // Komárom-Esztergom
		Borsod = 8,                // Borsod
		HajduBihar = 9,            // Hajdú-Bihar
		Bekes = 10,                // Békés
		Csongrad = 11,             // Csongrád
		Tolna = 12,                // Tolna
		Somogy = 13,               // Somogy
		Veszprem = 14,             // Veszprém
		GyorMosonSopron = 15,      // Győr-Moson-Sopron
		SzabolcsSzatmarBereg = 16, // Szabolcs-Szatmár-Bereg
		Baranya = 17,              // Baranya
		Zala = 18,                 // Zala
		Vas = 19,                  // Vas
	}

	// merge the two
	pub(crate) fn county_serialize<S>(counties: &HashSet<County>, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// there might be more efficient methods than copying but this works for now
		let res = counties.iter().map(|&county| county as i32).collect();
		s.serialize_str(&encode_available_areas(res))
	}

	pub(crate) fn available_serialize<S>(
		counties: &Option<HashSet<County>>,
		s: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let counties = match counties {
			// todo return with error?
			None => return s.serialize_str("000000"),
			Some(s) => s,
		};
		// there might be more efficient methods than copying but this works for now
		let res = counties.iter().map(|&county| county as i32).collect();
		s.serialize_str(&encode_available_areas(res))
	}
	pub fn decode_available_areas(available: i32) -> Vec<i32> {
		let mut res = Vec::new();
		for i in 1..=30 {
			if (available & (1 << (i - 1))) != 0 {
				res.push(i);
			}
		}
		res
	}

	pub fn encode_available_areas(areas: Vec<i32>) -> String {
		let mut available: i32 = 0;

		for &area in &areas {
			if area >= 1 && area <= 30 {
				available |= 1 << (area - 1);
			}
		}

		// Convert the integer to a byte array (in big-endian format)
		let available_bytes = available.to_be_bytes();

		to_hex_with_length(&available_bytes, 6)
	}
}

#[derive(Debug)]
pub struct GameState {
	pub state: i32,
	pub gameround: i32,
	pub phase: i32,
}

impl Serialize for GameState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{},{}", self.state, self.gameround, self.phase);

		serializer.serialize_str(&s)
	}
}

#[derive(Debug)]
// todo find out what this is
struct RoundInfo {
	lpnum: i32,
	next_player: i32,
}

impl Serialize for RoundInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{},{}", self.lpnum, self.next_player);

		serializer.serialize_str(&s)
	}
}

#[derive(Debug)]
struct ShieldMission {
	shieldmission: i32,
	shieldmission_rt: i32,
}

impl Serialize for ShieldMission {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// hexadecimal
		let s = format!("{:X},{:X}", self.shieldmission, self.shieldmission_rt);

		serializer.serialize_str(&s)
	}
}

fn to_hex_with_length(bytes: &[u8], length: usize) -> String {
	let encoded = hex::encode(bytes);
	let trimmed = encoded.trim_start_matches('0');

	// Format the string to the desired length
	format!("{:0>width$}", trimmed, width = length).to_uppercase()
}
