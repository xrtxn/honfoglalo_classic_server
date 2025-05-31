use serde::Serialize;
use tokio_stream::StreamExt;

use crate::{
	emulator::Emulator,
	triviador::{game::SharedTrivGame, game_state::GameState, triviador_state::TriviadorState},
};

pub(crate) struct EndScreenHandler {
	game: SharedTrivGame,
}

impl EndScreenHandler {
	pub(crate) fn new(game: SharedTrivGame) -> Self {
		EndScreenHandler { game }
	}

	pub(crate) async fn handle_all(&self) {
		self.game.write().await.state.game_state = GameState {
			state: 15,
			round: 0,
			phase: 0,
		};
		let state = self.game.read().await.state.clone();
		let gameover = Gameover::emulate();
		let division = Division::emulate();

		let es = EndscreenHandlerResponse {
			state,
			gameover,
			division,
		};

		let utils = self.game.read().await.utils.clone();
		let mut iter = utils.active_players_stream();
		while let Some(player) = iter.next().await {
			self.game
				.send_xml_channel(player, quick_xml::se::to_string(&es).unwrap())
				.await
				.unwrap();
		}
	}
}

#[derive(Serialize)]
#[serde(rename = "ROOT")]
pub(crate) struct EndscreenHandlerResponse {
	#[serde(rename = "STATE")]
	pub state: TriviadorState,
	#[serde(rename = "GAMEOVER")]
	pub gameover: Gameover,
	#[serde(rename = "DIVISION")]
	pub division: Division,
}

#[derive(Serialize)]
pub struct Gameover {
	#[serde(rename = "@PLACINGS")]
	pub placings: String,
	#[serde(rename = "@ROOMID")]
	pub roomid: String,
	/// old xp count,total xp change,current xp level,level change 1/0 (true/false)
	#[serde(rename = "@XP")]
	pub xp: String,
	/// placement, base xp points
	#[serde(rename = "@XPPL")]
	pub xppl: String,
	/// point percent, bonus xp
	#[serde(rename = "@XPPP")]
	pub xppp: String,
	/// average opponent level, bonus xp
	#[serde(rename = "@XPOPP")]
	pub xpopp: String,
	/// clan win count, clan win bonus xp
	#[serde(rename = "@XPCW")]
	pub xpcw: String,
	/// clan win count 2, clan win bonus xp 2
	#[serde(rename = "@XPCW2")]
	pub xpcw2: String,
	/// All answers count, won answers count
	#[serde(rename = "@ANSWERS")]
	pub answers: String,
	#[serde(rename = "@TIPS")]
	/// All tips count, won tips count
	pub tips: String,
	/// tip vep count, tip vep total
	/// tip accuracy
	// Util.SetText(w.TIPRATIO.VALUE.FIELD,Math.round(100 * _data.tipveptotal / _data.tipvepcount / 100).toFixed(0) + "%");
	#[serde(rename = "@VEPTIPS")]
	pub veptips: String,
	#[serde(rename = "@GOLDS")]
	pub golds: String,
	#[serde(rename = "@RL")]
	pub rl: String,
}

#[derive(Serialize)]
pub struct Division {
	#[serde(rename = "@USERID")]
	pub userid: String,
	#[serde(rename = "@TOTALXP")]
	pub totalxp: String,
	#[serde(rename = "@GAMECOUNT")]
	pub gamecount: String,
	#[serde(rename = "@LEAGUE")]
	pub league: String,
	#[serde(rename = "@DIVISION")]
	pub division: String,
	#[serde(rename = "@CLOSETIME")]
	pub closetime: String,
	#[serde(rename = "@UPCOUNT")]
	pub upcount: String,
	#[serde(rename = "@DOWNCOUNT")]
	pub downcount: String,
	#[serde(rename = "MEMBER")]
	pub member: Vec<Member>,
}

#[derive(Serialize)]
pub struct Member {
	#[serde(rename = "@USERID")]
	pub userid: String,
	#[serde(rename = "@TOTALXP")]
	pub totalxp: String,
	#[serde(rename = "@GAMECOUNT")]
	pub gamecount: String,
	#[serde(rename = "@COUNTRY")]
	pub country: String,
}

impl Emulator for Gameover {
	fn emulate() -> Self {
		Gameover {
			placings: "123".to_string(),
			roomid: "1".to_string(),
			xp: "14000,895,15,0".to_string(),
			xppl: "1,300".to_string(),
			xppp: "33,100".to_string(),
			xpopp: "15,45".to_string(),
			xpcw: "2,200".to_string(),
			xpcw2: "5,250".to_string(),
			answers: "6,4".to_string(),
			tips: "5,2".to_string(),
			veptips: "4,200".to_string(),
			golds: "3000".to_string(),
			rl: "7894,8789,3149".to_string(),
		}
	}
}

impl Emulator for Division {
	fn emulate() -> Self {
		Division {
			userid: "-1".to_string(),
			totalxp: "2201".to_string(),
			gamecount: "4".to_string(),
			league: "7".to_string(),
			division: "5".to_string(),
			closetime: "1418860800".to_string(),
			upcount: "10".to_string(),
			downcount: "0".to_string(),
			member: vec![
				Member {
					userid: "-1".to_string(),
					totalxp: "2201".to_string(),
					gamecount: "4".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100000712460617".to_string(),
					totalxp: "816".to_string(),
					gamecount: "3".to_string(),
					country: "sc".to_string(),
				},
				Member {
					userid: "1115113337".to_string(),
					totalxp: "709".to_string(),
					gamecount: "2".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100000681330785".to_string(),
					totalxp: "478".to_string(),
					gamecount: "2".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100003815212032".to_string(),
					totalxp: "476".to_string(),
					gamecount: "2".to_string(),
					country: "in".to_string(),
				},
				Member {
					userid: "100003005631435".to_string(),
					totalxp: "475".to_string(),
					gamecount: "2".to_string(),
					country: "ak".to_string(),
				},
				Member {
					userid: "100000849591663".to_string(),
					totalxp: "473".to_string(),
					gamecount: "2".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100001048354089".to_string(),
					totalxp: "457".to_string(),
					gamecount: "2".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100000467678010".to_string(),
					totalxp: "409".to_string(),
					gamecount: "3".to_string(),
					country: "wv".to_string(),
				},
				Member {
					userid: "100000582034112".to_string(),
					totalxp: "350".to_string(),
					gamecount: "1".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100003049708245".to_string(),
					totalxp: "347".to_string(),
					gamecount: "1".to_string(),
					country: "--".to_string(),
				},
				Member {
					userid: "100000021832432".to_string(),
					totalxp: "343".to_string(),
					gamecount: "1".to_string(),
					country: "tx".to_string(),
				},
				Member {
					userid: "100003124131072".to_string(),
					totalxp: "340".to_string(),
					gamecount: "1".to_string(),
					country: "tn".to_string(),
				},
				Member {
					userid: "1755000119".to_string(),
					totalxp: "339".to_string(),
					gamecount: "1".to_string(),
					country: "fl".to_string(),
				},
				Member {
					userid: "44407108".to_string(),
					totalxp: "338".to_string(),
					gamecount: "1".to_string(),
					country: "tx".to_string(),
				},
				Member {
					userid: "100002951353496".to_string(),
					totalxp: "337".to_string(),
					gamecount: "1".to_string(),
					country: "ks".to_string(),
				},
				Member {
					userid: "100000085106314".to_string(),
					totalxp: "333".to_string(),
					gamecount: "1".to_string(),
					country: "--".to_string(),
				},
			],
		}
	}
}
