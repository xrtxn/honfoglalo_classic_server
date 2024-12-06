use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;
use tracing::error;

use crate::emulator::Emulator;
use crate::triviador::cmd::Cmd;
use crate::triviador::triviador_state::TriviadorState;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename = "ROOT")]
pub(crate) struct QuestionStageResponse {
	#[serde(rename = "STATE")]
	pub state: TriviadorState,
	#[serde(rename = "QUESTION")]
	pub question: Option<Question>,
	#[serde(rename = "CMD")]
	pub cmd: Option<Cmd>,
	#[serde(rename = "ANSWERRESULT")]
	pub answer_result: Option<QuestionAnswerResult>,
}

impl QuestionStageResponse {
	pub(crate) fn new_question(state: TriviadorState, question: Question) -> QuestionStageResponse {
		QuestionStageResponse {
			state,
			question: Some(question),
			cmd: Some(Cmd::answer_command(20)),
			answer_result: None,
		}
	}

	pub(crate) fn new_answer_result(
		state: TriviadorState,
		answer_result: QuestionAnswerResult,
	) -> QuestionStageResponse {
		QuestionStageResponse {
			state,
			question: None,
			cmd: None,
			answer_result: Some(answer_result),
		}
	}
}

/// This serialized struct gets sent after all players gave their answer
/// (the reason why is a mystery)
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "ANSWER")]
pub struct SelfAnswer {
	#[serde(rename = "@ANSWER")]
	answer: u8,
}

impl SelfAnswer {
	pub(crate) fn new(answer: u8) -> SelfAnswer {
		SelfAnswer { answer }
	}

	pub(crate) fn get_answer(&self) -> u8 {
		self.answer
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Question {
	#[serde(rename = "@QUESTION")]
	pub question: String,
	#[serde(rename = "@ALLOWMARK")]
	pub allowmark: String,
	#[serde(rename = "@THEME")]
	pub theme: String,
	#[serde(rename = "@OP1")]
	pub option_1: String,
	#[serde(rename = "@OP2")]
	pub option_2: String,
	#[serde(rename = "@OP3")]
	pub option_3: String,
	#[serde(rename = "@OP4")]
	pub option_4: String,
	#[serde(rename = "@ICON_URL")]
	pub icon_url: String,
	#[serde(rename = "@COLOR_CODE")]
	pub color_code: String,
	#[serde(rename = "@HELP")]
	pub help: String,
}

impl Question {
	pub(crate) fn new(
		question: String,
		opt_1: String,
		opt_2: String,
		opt_3: String,
		opt_4: String,
	) -> Question {
		Question {
			question,
			allowmark: "1".to_string(),
			theme: "3".to_string(),
			option_1: opt_1,
			option_2: opt_2,
			option_3: opt_3,
			option_4: opt_4,
			icon_url: "client/assets/icons/pokeball.png".to_string(),
			color_code: "F3C5C3".to_string(),
			// {HALF:2000,ANSWERS:2000}
			help: "{}".to_string(),
		}
	}
}

impl Emulator for Question {
	fn emulate() -> Self {
		Question::new(
			"Which of these is a Pokemon?".to_string(),
			"Charmander".to_string(),
			"Digimon".to_string(),
			"Yugioh".to_string(),
			"Dragonball".to_string(),
		)
	}
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QuestionAnswerResult {
	#[serde(rename = "@PLAYER1")]
	pub player1: Option<u8>,
	#[serde(rename = "@PLAYER2")]
	pub player2: Option<u8>,
	#[serde(rename = "@PLAYER3")]
	pub player3: Option<u8>,
	#[serde(rename = "@GOOD")]
	pub good: Option<u8>,
}

impl QuestionAnswerResult {
	pub(crate) fn new() -> QuestionAnswerResult {
		QuestionAnswerResult {
			player1: None,
			player2: None,
			player3: None,
			good: None,
		}
	}

	pub(crate) fn set_player(&mut self, rel_id: u8, player_answer: u8) {
		match rel_id {
			1 => self.player1 = Some(player_answer),
			2 => self.player2 = Some(player_answer),
			3 => self.player3 = Some(player_answer),
			_ => {
				error!("Unable to set player answer, invalid player id: {}", rel_id);
			}
		}
	}

	pub(crate) fn get_player(&self, rel_id: u8) -> Option<u8> {
		match rel_id {
			1 => self.player1,
			2 => self.player2,
			3 => self.player3,
			_ => {
				error!("Unable to get player answer, invalid player id: {}", rel_id);
				None
			}
		}
	}

	pub(crate) fn is_player_correct(&self, rel_id: u8) -> bool {
		let correct_answer;
		match self.good {
			None => {
				error!("Unable to check if answer is correct, good answer not set, setting to placeholder 1");
				correct_answer = 1;
			}
			Some(_) => {
				correct_answer = self.good.unwrap();
			}
		}
		match self.get_player(rel_id) {
			Some(player_answer) => player_answer == correct_answer,
			None => false,
		}
	}
}

#[skip_serializing_none]
#[derive(Serialize, Debug)]
#[serde(rename = "ROOT")]
pub struct TipStageResponse {
	#[serde(rename = "STATE")]
	pub state: TriviadorState,
	#[serde(rename = "CMD")]
	pub cmd: Option<Cmd>,
	#[serde(rename = "TIPQUESTION")]
	pub question: Option<Question>,
	#[serde(rename = "TIPINFO")]
	pub tip_info: Option<TipInfo>,
	#[serde(rename = "TIPRESULT")]
	pub tip_result: Option<TipResult>,
}

impl TipStageResponse {
	pub(crate) fn new(state: TriviadorState) -> TipStageResponse {
		TipStageResponse {
			state,
			cmd: None,
			question: None,
			tip_info: None,
			tip_result: None,
		}
	}

	pub(crate) fn new_tip(state: TriviadorState, question: Question) -> TipStageResponse {
		TipStageResponse {
			state,
			cmd: Some(Cmd::tip_command(15)),
			question: Some(question),
			tip_info: None,
			tip_result: None,
		}
	}

	pub(crate) fn new_tip_result(
		state: TriviadorState,
		tip_info: TipInfo,
		good: i32,
	) -> TipStageResponse {
		let mut results: HashMap<u8, i32> = HashMap::new();
		results.insert(1, TipInfo::difference(good, tip_info.player_1_tip.unwrap()));
		results.insert(2, TipInfo::difference(good, tip_info.player_2_tip.unwrap()));
		results.insert(3, TipInfo::difference(good, tip_info.player_3_tip.unwrap()));

		let mut sorted_results: Vec<_> = results.iter().collect();
		sorted_results.sort_by(|a, b| a.1.cmp(b.1));

		TipStageResponse {
			state,
			cmd: None,
			question: None,
			tip_info: Some(tip_info),
			tip_result: Some(TipResult {
				winner: *sorted_results[0].0,
				second: *sorted_results[1].0,
				good,
			}),
		}
	}
}

// floats are cut to 3 decimal places
// closeness is similar to percentage (100-1)
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TipInfo {
	#[serde(rename = "@TIMEORDER")]
	// the order of player answers (123)
	#[serde(serialize_with = "timeorder_serializer")]
	pub timeorder: Vec<u8>,
	#[serde(rename = "@T1")]
	pub player_1_time: Option<f32>,
	#[serde(rename = "@V1")]
	pub player_1_tip: Option<i32>,
	#[serde(rename = "@A1")]
	pub player_1_closeness: Option<String>,
	#[serde(rename = "@T2")]
	pub player_2_time: Option<f32>,
	#[serde(rename = "@V2")]
	pub player_2_tip: Option<i32>,
	#[serde(rename = "@A2")]
	pub player_2_closeness: Option<String>,
	#[serde(rename = "@T3")]
	pub player_3_time: Option<f32>,
	#[serde(rename = "@V3")]
	pub player_3_tip: Option<i32>,
	#[serde(rename = "@A3")]
	pub player_3_closeness: Option<String>,
}

fn timeorder_serializer<S>(x: &Vec<u8>, s: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	s.serialize_str(&x.iter().map(|x| x.to_string()).collect::<String>())
}

impl TipInfo {
	pub(crate) fn new() -> TipInfo {
		TipInfo {
			timeorder: Vec::with_capacity(3),
			player_1_time: None,
			player_1_tip: None,
			player_1_closeness: None,
			player_2_time: None,
			player_2_tip: None,
			player_2_closeness: None,
			player_3_time: None,
			player_3_tip: None,
			player_3_closeness: None,
		}
	}

	pub(crate) fn add_player_tip(&mut self, rel_id: u8, tip: i32, time: f32) {
		self.timeorder.push(rel_id);
		// todo implement closeness calculation
		match rel_id {
			1 => {
				self.player_1_tip = Some(tip);
				self.player_1_time = Some(time);
				self.player_1_closeness = Some("10".to_string());
			}
			2 => {
				self.player_2_tip = Some(tip);
				self.player_2_time = Some(time);
				self.player_2_closeness = Some("90".to_string());
			}
			3 => {
				self.player_3_tip = Some(tip);
				self.player_3_time = Some(time);
				self.player_3_closeness = Some("90".to_string());
			}
			_ => {
				error!("Unable to set player tip, invalid player id: {}", rel_id);
			}
		}
	}
	pub(crate) fn difference(good: i32, answer: i32) -> i32 {
		(good - answer).abs()
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct TipResult {
	#[serde(rename = "@WINNER")]
	pub winner: u8,
	#[serde(rename = "@SECOND")]
	pub second: u8,
	#[serde(rename = "@GOOD")]
	pub good: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "TIP")]
pub struct PlayerTipResponse {
	#[serde(rename = "@TIP")]
	pub tip: i32,
	#[serde(rename = "@HUMAN")]
	pub human: bool,
}
