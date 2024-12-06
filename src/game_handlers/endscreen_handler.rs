use serde::{Deserialize, Serialize};

use crate::emulator::Emulator;

#[derive(Serialize, Deserialize)]
pub struct EndscreenHandlerRoot {
	#[serde(rename = "GAMEOVER")]
	pub gameover: Gameover,
	#[serde(rename = "DIVISION")]
	pub division: Division,
}

#[derive(Serialize, Deserialize)]
pub struct Gameover {
	#[serde(rename = "@PLACINGS")]
	pub placings: String,
	#[serde(rename = "@ROOMID")]
	pub roomid: String,
	#[serde(rename = "@XP")]
	pub xp: String,
	#[serde(rename = "@XPPL")]
	pub xppl: String,
	#[serde(rename = "@XPPP")]
	pub xppp: String,
	#[serde(rename = "@XPOPP")]
	pub xpopp: String,
	#[serde(rename = "@XPCW")]
	pub xpcw: String,
	#[serde(rename = "@XPCW2")]
	pub xpcw2: String,
	#[serde(rename = "@ANSWERS")]
	pub answers: String,
	#[serde(rename = "@TIPS")]
	pub tips: String,
	#[serde(rename = "@VEPTIPS")]
	pub veptips: String,
	#[serde(rename = "@GOLDS")]
	pub golds: String,
	#[serde(rename = "@RL")]
	pub rl: String,
}

#[derive(Serialize, Deserialize)]
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
	#[serde(rename = "$text")]
	pub text: Option<String>,
	#[serde(rename = "MEMBER")]
	pub member: Vec<Member>,
}

#[derive(Serialize, Deserialize)]
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

impl Emulator for EndscreenHandlerRoot {
	fn emulate() -> Self {
		EndscreenHandlerRoot {
			gameover: Gameover {
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
			},
			division: Division {
				userid: "-1".to_string(),
				totalxp: "2201".to_string(),
				gamecount: "4".to_string(),
				league: "7".to_string(),
				division: "5".to_string(),
				closetime: "1418860800".to_string(),
				upcount: "10".to_string(),
				downcount: "0".to_string(),
				text: None,
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
			},
		}
	}
}
