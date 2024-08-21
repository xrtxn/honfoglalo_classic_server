use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Query;
use axum::{extract, Extension, Json};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::app::SinglePlayerState;
use crate::cdn::countries::CountriesResponse;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::ChannelType;
use crate::emulator::{remove_root_tag, HungaryEmulator};
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::mobile::request::Mobile;
use crate::mobile::response::{LoginResponse, MobileResponse, PingResponse};
use crate::village::castle::badges::CastleResponse;
use crate::village::setup::VillageSetupRoot;
use crate::village::waithall::GameMenuWaithall;

pub async fn help(_pool: Extension<PgPool>) -> Json<HelpResponse> {
	// todo find out how to use this
	Json(HelpResponse::emulate("".to_string()))
}

pub async fn countries(_pool: Extension<PgPool>) -> Json<CountriesResponse> {
	Json(CountriesResponse::emulate("".to_string()))
}
pub async fn friends(_pool: Extension<PgPool>) -> Json<FriendResponse> {
	Json(FriendResponse::emulate("".to_string()))
}

pub async fn extdata(_pool: Extension<PgPool>) -> Json<FriendResponse> {
	Json(FriendResponse::emulate("".to_string()))
}

pub async fn mobil(_pool: Extension<PgPool>, Json(payload): Json<Mobile>) -> Json<MobileResponse> {
	match payload {
		Mobile::Ping(_) => Json(MobileResponse::Ping(PingResponse {
			message: "pong".to_string(),
		})),
		Mobile::MobileLogin(_) => Json(MobileResponse::Login(LoginResponse::emulate(
			"".to_string(),
		))),
	}
}

pub async fn game(
	_pool: Extension<PgPool>,
	state: extract::State<Arc<Mutex<SinglePlayerState>>>,
	headers: Query<HashMap<String, String>>,
	body: String,
) -> String {
	let string = {
		let lines: Vec<&str> = body.lines().collect();
		format!("<ROOT>{}</ROOT>", lines.get(1).unwrap())
	};
	let headers = {
		let serialized_headers = serde_json::to_string(&headers.0).unwrap();
		serde_json::from_str::<ChannelType>(&serialized_headers).unwrap()
	};
	match headers {
		ChannelType::Command(comm) => {
			let ser: CommandRoot = quick_xml::de::from_str(&string).unwrap();
			match ser.msg_type {
				CommandType::Login(_) => {
					let mut t_state = state.0.lock().await;
					t_state.is_logged_in = false;
					t_state.listen_queue = VecDeque::new();
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
				CommandType::ChangeWaitHall(chw) => {
					// todo match chw
					let xml = remove_root_tag(
						quick_xml::se::to_string(&GameMenuWaithall::emulate(comm.mn.clone()))
							.unwrap(),
					);
					state.0.lock().await.listen_queue.push_back(xml);
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
				CommandType::EnterGameLobby(lobby) => {
					let xml = remove_root_tag(todo!());
					state.0.lock().await.listen_queue.push_back(xml);
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
				CommandType::GetExternalData(_) => {
					let xml = format!(
						r#"<C CID="1" MN="{}" R="0" />
                                    <EXTDATA>
                                        <USER>
                                            <ID>2</ID>
                                            <NAME>foo</NAME>
                                            <USECUSTOM>0</USECUSTOM>
                                            <CUSTOM>todo</CUSTOM>
                                            <IMGURL>//graph.facebook.com/1/picture</IMGURL>
                                            <ONLINE>1</ONLINE>
                                        </USER>
                                        <USER>
                                            <ID>3</ID>
                                            <NAME>bar</NAME>
                                            <USECUSTOM>0</USECUSTOM>
                                            <CUSTOM>todo</CUSTOM>
                                            <IMGURL>//graph.facebook.com/1/picture</IMGURL>
                                            <ONLINE>1</ONLINE>
                                        </USER>
                                    </EXTDATA>
"#,
						comm.mn
					);
					xml.to_string()
				}
				CommandType::ExitCurrentRoom(_) => {
					println!("Encountered stub ExitCurrentRoom, this response may work or may not");
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
				CommandType::AddFriendlyRoom(f_room) => {
					let xml = format!(
						r#"<L CID="1" MN="{}" R="0"/>
                        <ACTIVESEPROOM CODE="1234"
                        P1="1,0" PN1="xrtxn" P2="-1,0" P3="-1,0"/>
                        "#,
						2
					);
					state.0.lock().await.listen_queue.push_back(xml);
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
				CommandType::StartFriendlyRoom(_) => {
					let xml = format!(
						r#"
                                <L CID="1" MN="{}" R="0"/>
                                <STATE SCR="MAP_WD" ST="11,1,0" CP="0,0" HC="123" CHS="0,0,0" PTS="0,0,0" SEL="000000" B="000000" A="0000000000000000000000000000000000000000" AA="000000" UH="0"/>
                                <PLAYERS P1="xrtxn" P2="null" P3="null" PD1="-1,14000,15,1,0,ar,1,,0" PD2="-1,14000,15,1,0,hu,1,,8" PD3="-1,14000,15,1,0,ar,1,,6" YOU="1,2,3" GAMEID="1" ROOM="1" RULES="0,0"/>
                        "#,
						4
					);
					state.0.lock().await.listen_queue.push_back(xml);
					quick_xml::se::to_string(&CommandResponse::emulate(comm.mn)).unwrap()
				}
			}
		}
		ChannelType::Listen(lis) => {
			let mut t_state = state.0.lock().await;
			if !t_state.is_logged_in {
				t_state.is_logged_in = true;
				return remove_root_tag(
					quick_xml::se::to_string(&VillageSetupRoot::emulate(lis.mn)).unwrap(),
				);
			}
			drop(t_state);
			loop {
				let mut vec = state.0.lock().await;
				if !vec.listen_queue.is_empty() {
					if lis.mn == "2" {
						println!("INSERTING NOW");
						// tokio::time::sleep(Duration::from_millis(2000)).await;
						let xml = format!(
							r#"<L CID="1" MN="{}" R="0"/>
                        <ACTIVESEPROOM CODE="1234" STARTDELAY="1"
                        P1="1,0" PN1="xrtxn" P2="-1,1" PN2="teszt1" P3="-1,1" PN3="teszt2" />
                        "#,
							3
						);
						vec.listen_queue.push_back(xml);
					}
					return vec.listen_queue.pop_front().unwrap();
				}
				drop(vec);

				tokio::time::sleep(Duration::from_millis(1000)).await;
			}
		}
	}
}

#[axum::debug_handler]
pub async fn client_castle(_pool: Extension<PgPool>) -> Json<CastleResponse> {
	Json(CastleResponse::emulate("".to_string()))
}
