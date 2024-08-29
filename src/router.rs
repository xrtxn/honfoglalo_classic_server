use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Query;
use axum::{extract, Extension, Json};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::app::SinglePlayerState;
use crate::cdn::countries::CountriesResponse;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::{CommandResponse, CommandResponseHeader};
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseType::VillageSetup;
use crate::channels::listen::response::{ListenResponse, ListenResponseHeader, ListenResponseType};
use crate::channels::ChannelType;
use crate::emulator::{remove_root_tag, Emulator};
use crate::menu::friend_list::external_data::ExternalFriendsRoot;
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::mobile::request::Mobile;
use crate::mobile::response::{LoginResponse, MobileResponse, PingResponse};
use crate::village::castle::badges::CastleResponse;
use crate::village::setup::VillageSetupRoot;
use crate::village::start::friendly_game::ActiveSepRoom;
use crate::village::waithall::GameMenuWaithall;

pub async fn help() -> Json<HelpResponse> {
	// todo find out how to use this
	Json(HelpResponse::emulate())
}

pub async fn countries() -> Json<CountriesResponse> {
	Json(CountriesResponse::emulate())
}
pub async fn friends() -> Json<FriendResponse> {
	Json(FriendResponse::emulate())
}

pub async fn extdata() -> Json<FriendResponse> {
	Json(FriendResponse::emulate())
}

pub async fn mobil(Json(payload): Json<Mobile>) -> Json<MobileResponse> {
	match payload {
		Mobile::Ping(_) => Json(MobileResponse::Ping(PingResponse {
			message: "pong".to_string(),
		})),
		Mobile::MobileLogin(_) => Json(MobileResponse::Login(LoginResponse::emulate())),
	}
}

pub async fn game(
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
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok("1", comm.mn)).unwrap(),
					)
				}
				CommandType::ChangeWaitHall(chw) => {
					// todo match chw
					state
						.0
						.lock()
						.await
						.listen_queue
						.push_back(quick_xml::se::to_string(&GameMenuWaithall::emulate()).unwrap());
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
					)
				}
				CommandType::EnterGameLobby(lobby) => {
					let xml = todo!();
					state.0.lock().await.listen_queue.push_back(xml);
					quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn)).unwrap()
				}
				CommandType::GetExternalData(_) => {
					let msg = quick_xml::se::to_string(&ExternalFriendsRoot::emulate()).unwrap();
					remove_root_tag(format!(
						"{}\n{}",
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
						msg
					))
				}
				CommandType::ExitCurrentRoom(_) => {
					warn!("Encountered stub ExitCurrentRoom, this response may work or may not");
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
					)
				}
				CommandType::AddFriendlyRoom(f_room) => {
					let xml = quick_xml::se::to_string(&ActiveSepRoom::new_bots_room(
						1,
						"xrtxn".to_string(),
					))
					.unwrap();
					state.0.lock().await.listen_queue.push_back(xml);
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
					)
				}
				CommandType::StartTriviador(_) => {
					let xml =
						quick_xml::se::to_string(&state.0.lock().await.triviador_state).unwrap();
					state.0.lock().await.listen_queue.push_back(xml);
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
					)
				}
				CommandType::PlayerReady => {
					// if state.0.lock().await.animation_finished {
					// 	tokio::time::sleep(Duration::from_millis(2000)).await;
					// 	return remove_root_tag(
					// 		quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
					// 			.unwrap(),
					// 	);
					// }
					let mut sps = state.0.lock().await;
					sps.is_listen_ready = true;
					match sps.triviador_state.state.game_state.state {
						11 => sps.triviador_state.announcement(),
						1 => match sps.triviador_state.state.game_state.phase {
							0 => sps.triviador_state.choose_area(),
							1 => {
								warn!("AC: not starting a timer here stops the game for everyone");
								return remove_root_tag(
									quick_xml::se::to_string(&CommandResponse::ok(
										comm.client_id,
										comm.mn,
									))
									.unwrap(),
								);
							}
							_ => {
								todo!()
							}
						},
						_ => {
							todo!()
						}
					}
					let xml = quick_xml::se::to_string(&sps.triviador_state).unwrap();
					sps.listen_queue.push_back(xml);
					sps.animation_finished = true;
					remove_root_tag(
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))
							.unwrap(),
					)
				}
			}
		}
		ChannelType::Listen(lis) => {
			let ser: ListenRoot = quick_xml::de::from_str(&string).unwrap();
			state.0.lock().await.is_listen_ready = ser.listen.is_ready;
			if !ser.listen.is_ready {
				while !state.0.lock().await.is_listen_ready {
					tokio::time::sleep(Duration::from_millis(1000)).await;
				}
			}
			let mut t_state = state.0.lock().await;
			if !t_state.is_logged_in {
				t_state.is_logged_in = true;
				return remove_root_tag(
					quick_xml::se::to_string(&ListenResponse::new(
						ListenResponseHeader {
							client_id: lis.client_id,
							mn: lis.mn,
							result: 0,
						},
						VillageSetup(VillageSetupRoot::emulate()),
					))
					.unwrap(),
				);
			}
			drop(t_state);
			loop {
				let mut vec = state.0.lock().await;
				if !vec.listen_queue.is_empty() {
					if lis.mn == "2" {
						let mut t = ActiveSepRoom::new_bots_room(1, "xrtxn".to_string());
						t.start_friendly_room();
						vec.listen_queue
							.push_back(quick_xml::se::to_string(&t).unwrap());
						tokio::time::sleep(Duration::from_millis(1000)).await;
					}

					return format!(
						"{}\n{}",
						quick_xml::se::to_string(&ListenResponseHeader {
							client_id: "1".to_string(),
							mn: lis.mn,
							result: 0,
						})
						.unwrap(),
						remove_root_tag(vec.listen_queue.pop_front().unwrap())
					);
				}
				drop(vec);

				tokio::time::sleep(Duration::from_millis(1000)).await;
			}
		}
	}
}

pub async fn client_castle() -> Json<CastleResponse> {
	Json(CastleResponse::emulate())
}
