use std::time::Duration;

use axum::{Extension, Json};
use fred::clients::RedisPool;
use scc::Queue;
use sqlx::PgPool;
use tokio::sync::{mpsc};
use tracing::{trace, warn};

use crate::app::{AppError, SharedPlayerChannel, SharedPlayerState};
use crate::cdn::countries::CountriesResponse;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseType::VillageSetup;
use crate::channels::listen::response::{ListenResponse, ListenResponseHeader};
use crate::channels::BodyChannelType;
use crate::emulator::Emulator;
use crate::game_handlers::server_game_handler::ServerGameHandler;
use crate::menu::friend_list::external_data::ExternalFriendsRoot;
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::mobile::request::Mobile;
use crate::mobile::response::{LoginResponse, MobileResponse, PingResponse};
use crate::users::ServerCommand;
use crate::utils::{modified_xml_response, remove_root_tag};
use crate::village::castle::badges::CastleResponse;
use crate::village::setup::VillageSetupRoot;
use crate::village::start::friendly_game::ActiveSepRoom;
use crate::village::waithall::{GameMenuWaithall, Waithall};

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

pub async fn _extdata() -> Json<FriendResponse> {
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

#[axum::debug_handler]
pub async fn game(
	_db: Extension<PgPool>,
	tmp_db: Extension<RedisPool>,
	xml_header: Extension<BodyChannelType>,
	player_state: Extension<SharedPlayerState>,
	// friendly_rooms: Extension<FriendlyRooms>,
	broadcast: Extension<SharedPlayerChannel<String>>,
	body: String,
) -> Result<String, AppError> {
	trace!("game handler called");
	const GAME_ID: u32 = 1;
	const PLAYER_ID: i32 = 1;
	const PLAYER_NAME: &str = "xrtxn";

	let body = format!("<ROOT>{}</ROOT>", body);
	match xml_header.0 {
		BodyChannelType::Command(comm) => {
			let ser: CommandRoot = quick_xml::de::from_str(&body)?;
			match ser.msg_type {
				CommandType::Login(_) => {
					// todo validate login
					player_state.set_login(false).await;
					player_state.set_listen_ready(false).await;
					player_state.set_server_command(None).await;
					Ok(modified_xml_response(&CommandResponse::ok(
						PLAYER_ID, comm.mn,
					))?)
				}
				CommandType::ChangeWaitHall(chw) => {
					let msg;
					match chw.waithall {
						Waithall::Game => {
							msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
						}
						Waithall::Village => {
							msg = quick_xml::se::to_string(&VillageSetupRoot::emulate())?;
						}
					}
					if broadcast.is_user_listening().await {
						trace!("sending message to channel");
						broadcast.send_message(msg).await.unwrap();
						trace!("sent message to channel");
					} else {
						trace!("pushing to listen");
						// player_state.push_listen(msg).await;
						trace!("pushed to listen");
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::EnterGameLobby(_) => {
					Ok(modified_xml_response(&CommandResponse::error())?)
				}
				CommandType::GetExternalData(_) => {
					let msg = quick_xml::se::to_string(&ExternalFriendsRoot::emulate())?;
					Ok(remove_root_tag(format!(
						"{}\n{}",
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))?,
						msg
					)))
				}
				CommandType::ExitCurrentRoom(_) => {
					warn!("Encountered stub ExitCurrentRoom, this response may work or may not");
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::AddFriendlyRoom(room) => {
					// todo handle other cases
					if room.opp1 == -1 && room.opp2 == -1 {
						// todo get next number
						let room_number = 1;
						// friendly_rooms.insert(
						// room_number,
						// ActiveSepRoom::new_bot_room(PLAYER_ID, PLAYER_NAME),
						// );
						//
						// let xml = quick_xml::se::to_string(&friendly_rooms.0)?;
						// broadcast.send_message(xml).await.unwrap();
					} else {
						return Ok(modified_xml_response(&CommandResponse::error())?);
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::StartTriviador(_) => {
					tokio::spawn(async move {
						ServerGameHandler::new_friendly(&tmp_db, GAME_ID).await;
					});
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::GamePlayerReady => {
					// player_state.set_listen_ready(true).await;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::SelectArea(area_selection) => {
					player_state
						.set_server_command(Some(ServerCommand::SelectArea(area_selection.area)))
						.await;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::QuestionAnswer(ans) => {
					player_state
						.set_server_command(Some(ServerCommand::QuestionAnswer(ans.answer)))
						.await;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::PlayerTipResponse(tip) => {
					player_state
						.set_server_command(Some(ServerCommand::TipAnswer(tip.tip)))
						.await;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
			}
		}
		BodyChannelType::Listen(lis) => {
			let ser: ListenRoot = quick_xml::de::from_str(&body)?;

			player_state.set_listen_ready(ser.listen.is_ready).await;

			trace!("listen state is {}", player_state.get_listen_ready().await);
			if !player_state.get_login().await {
				player_state.set_login(true).await;
				player_state.set_listen_ready(false).await;
				return Ok(modified_xml_response(&ListenResponse::new(
					ListenResponseHeader {
						client_id: lis.client_id,
						mn: lis.mn,
						result: 0,
					},
					VillageSetup(VillageSetupRoot::emulate()),
				))?);
			}

			let next_listen = player_state.get_next_listen().await;
			if next_listen.is_none() {
				trace!("subbed to recv message");
				let rx = broadcast.new_receiver().await;
				trace!("got new rx");
				let msg = SharedPlayerChannel::recv_message(rx).await.unwrap();
				trace!("recv'd message");
				Ok(format!(
					"{}\n{}",
					quick_xml::se::to_string(&ListenResponseHeader {
						client_id: PLAYER_ID,
						mn: lis.mn,
						result: 0,
					})?,
					remove_root_tag(msg)
				))
			} else {
				Ok(format!(
					"{}\n{}",
					quick_xml::se::to_string(&ListenResponseHeader {
						client_id: PLAYER_ID,
						mn: lis.mn,
						result: 0,
					})?,
					remove_root_tag(next_listen.unwrap())
				))
			}
		}
	}
}

pub async fn client_castle() -> Json<CastleResponse> {
	Json(CastleResponse::emulate())
}
