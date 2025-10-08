use axum::{Extension, Json};
use sqlx::PgPool;

use crate::app::{
	AppError, FriendlyRooms, ServerCommandChannel, SharedPlayerState, XmlPlayerChannel,
};
use crate::cdn::countries::CountriesResponse;
use crate::channels::BodyChannelType;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::heartbeat::request::response::HeartBeatResponse;
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseHeader;
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
use crate::village::start::friendly_game::OpponentType;
use crate::village::waithall::{GameMenuWaithall, Waithall};

const QUICK_BATTLE_EMU: bool = false;

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
	db: Extension<PgPool>,
	xml_header: Extension<BodyChannelType>,
	player_state: Extension<SharedPlayerState>,
	friendly_rooms: Extension<FriendlyRooms>,
	player_listen_channel: Extension<XmlPlayerChannel>,
	server_command_channel: Extension<ServerCommandChannel>,
	body: String,
) -> Result<String, AppError> {
	//todo fetch this from db
	const GAME_ID: u32 = 1;
	const PLAYER_ID: OpponentType = OpponentType::Player(1);
	const PLAYER_NAME: &str = "xrtxn";

	let body = format!("<ROOT>{}</ROOT>", body);
	match xml_header.0 {
		BodyChannelType::Command(comm) => {
			let ser: CommandRoot = quick_xml::de::from_str(&body)?;
			match ser.msg_type {
				CommandType::Login(_) => {
					// todo validate login
					player_state.set_login(true).await;
					player_state.set_listen_ready(false).await;
					player_listen_channel
						.send_message(quick_xml::se::to_string(&VillageSetupRoot::emulate())?)
						.await
						.unwrap();
					if QUICK_BATTLE_EMU {
						tokio::spawn(async move {
							ServerGameHandler::new_friendly(
								player_listen_channel.0,
								server_command_channel.0,
								GAME_ID,
								db.0,
							)
							.await;
						});
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						PLAYER_ID.get_id(),
						comm.mn,
					))?)
				}
				CommandType::ChangeWaitHall(chw) => {
					player_state.set_current_waithall(chw.waithall).await;

					let msg = match chw.waithall {
						Waithall::Game => quick_xml::se::to_string(&GameMenuWaithall::emulate())?,
						Waithall::Village => {
							quick_xml::se::to_string(&VillageSetupRoot::emulate())?
						}
					};
					player_listen_channel.send_message(msg).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::EnterGameLobby(_) => {
					Ok(modified_xml_response(&CommandResponse::error())?)
				}
				CommandType::GetExternalData(_) => {
					if player_state.get_current_waithall().await == Waithall::Game {

					}

					let msg = quick_xml::se::to_string(&ExternalFriendsRoot::emulate())?;
					Ok(remove_root_tag(format!(
						"{}\n{}",
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))?,
						msg
					)))
				}
				CommandType::ExitCurrentRoom(_) => Ok(modified_xml_response(
					&CommandResponse::ok(comm.client_id, comm.mn),
				)?),
				CommandType::CloseGame => {
					//send back to the menu
					let msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
					player_listen_channel.send_message(msg).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				// This gets called after clicking on the button add players to the friendly room
				CommandType::AddFriendlyRoom(request_room) => {
					use OpponentType::*;

					let room_number = friendly_rooms.len() as u16;

					friendly_rooms
						.insert(room_number, ActiveSepRoom::new(PLAYER_ID, PLAYER_NAME))
						.unwrap();

					let mut room = friendly_rooms.get(&room_number).unwrap();

					room.add_opponent(request_room.opp1);
					room.add_opponent(request_room.opp2);

					//todo move this
					if matches!(request_room.opp1, Robot)
						|| matches!(request_room.opp1, Player(_))
							&& matches!(request_room.opp2, Robot)
					{
						room.allow_game();
					}

					drop(room);

					let xml = quick_xml::se::to_string(
						&friendly_rooms
							.0
							.read(&room_number, |_, v| v.clone())
							.unwrap(),
					)?;
					player_listen_channel.send_message(xml).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::JoinFriendlyRoom(room) => {
					if room.code.is_some() {
						let xml = quick_xml::se::to_string(
							&friendly_rooms
								.0
								.read(&room.code.unwrap(), |_, v| v.clone())
								.unwrap(),
						)?;
						player_listen_channel.send_message(xml).await.unwrap();
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
						ServerGameHandler::new_friendly(
							player_listen_channel.0,
							server_command_channel.0,
							GAME_ID,
							db.0,
						)
						.await;
					});
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::GamePlayerReady => {
					server_command_channel
						.send_message(ServerCommand::Ready)
						.await?;
					player_state.set_listen_ready(true).await;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::SelectArea(area_selection) => {
					server_command_channel
						.send_message(ServerCommand::SelectArea(area_selection.area))
						.await?;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::QuestionAnswer(ans) => {
					server_command_channel
						.send_message(ServerCommand::QuestionAnswer(ans.get_answer()))
						.await?;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::PlayerTipResponse(tip) => {
					server_command_channel
						.send_message(ServerCommand::TipAnswer(tip.tip))
						.await?;

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

			let msg = player_listen_channel.recv_message().await.unwrap();
			Ok(format!(
				"{}\n{}",
				quick_xml::se::to_string(&ListenResponseHeader {
					client_id: lis.client_id,
					mn: lis.mn,
					result: 0,
				})?,
				remove_root_tag(msg)
			))
		}
		BodyChannelType::HeartBeat(hb) => Ok(modified_xml_response(&HeartBeatResponse::ok(
			hb.client_id,
			hb.mn,
		))?),
	}
}

pub async fn client_castle() -> Json<CastleResponse> {
	Json(CastleResponse::emulate())
}
