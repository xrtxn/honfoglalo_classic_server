use log::error;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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
	pub answer_result: Option<AnswerResult>,
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
		answer_result: AnswerResult,
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
			help: "{HALF:2000,ANSWERS:2000}".to_string(),
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
pub struct AnswerResult {
	#[serde(rename = "@PLAYER1")]
	pub player1: Option<u8>,
	#[serde(rename = "@PLAYER2")]
	pub player2: Option<u8>,
	#[serde(rename = "@PLAYER3")]
	pub player3: Option<u8>,
	#[serde(rename = "@GOOD")]
	pub good: Option<u8>,
}

impl AnswerResult {
	pub(crate) fn new() -> AnswerResult {
		AnswerResult {
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
