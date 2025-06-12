use axum::{Extension, Json};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use sqlx::PgPool;
use tracing::{trace, warn};

use crate::app::{
	AppError, FriendlyRooms, ServerCommandChannel, SharedPlayerState, XmlPlayerChannel,
};
use crate::cdn::countries::CountriesResponse;
use crate::channels::BodyChannelType;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::heartbeat::request::response::HeartBeatResponse;
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseType::VillageSetup;
use crate::channels::listen::response::{ListenResponse, ListenResponseHeader};
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
	_db: Extension<PgPool>,
	xml_header: Extension<BodyChannelType>,
	player_state: Extension<SharedPlayerState>,
	friendly_rooms: Extension<FriendlyRooms>,
	player_channel: Extension<XmlPlayerChannel>,
	server_command_channel: Extension<ServerCommandChannel>,
	body: String,
) -> Result<String, AppError> {
	//todo fetch this from db
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
					if QUICK_BATTLE_EMU {
						tokio::spawn(async move {
							ServerGameHandler::new_friendly(
								player_channel.0,
								server_command_channel.0,
								GAME_ID,
							)
							.await;
						});
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						PLAYER_ID, comm.mn,
					))?)
				}
				CommandType::ChangeWaitHall(chw) => {
					let msg = match chw.waithall {
						Waithall::Game => quick_xml::se::to_string(&GameMenuWaithall::emulate())?,
						Waithall::Village => {
							quick_xml::se::to_string(&VillageSetupRoot::emulate())?
						}
					};
					player_channel.send_message(msg).await.unwrap();
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
				CommandType::CloseGame => {
					//send back to the menu
					let msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
					player_channel.send_message(msg).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::AddFriendlyRoom(room) => {
					trace!("add friendly room");
					// todo handle other cases
					if room.opp1 == OpponentType::Robot && room.opp2 == OpponentType::Robot {
						// todo get next number
						let mut rng = StdRng::from_entropy();
						let room_number = rng.gen_range(1..=100000);
						friendly_rooms
							.insert(
								room_number,
								ActiveSepRoom::new_bot_room(PLAYER_ID, PLAYER_NAME),
							)
							.unwrap();
						trace!("friendly_rooms.0:{:?}", friendly_rooms.0);

						let xml = quick_xml::se::to_string(
							&friendly_rooms
								.0
								.read(&room_number, |_, v| v.clone())
								.unwrap(),
						)?;
						player_channel.send_message(xml).await.unwrap();
						trace!("friendly room added");
					} else {
						return Ok(modified_xml_response(&CommandResponse::error())?);
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::JoinFriendlyRoom(room) => Ok(modified_xml_response(
					&CommandResponse::ok(comm.client_id, comm.mn),
				)?),
				CommandType::StartTriviador(_) => {
					tokio::spawn(async move {
						ServerGameHandler::new_friendly(
							player_channel.0,
							server_command_channel.0,
							GAME_ID,
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

			let msg = player_channel.recv_message().await.unwrap();
			Ok(format!(
				"{}\n{}",
				quick_xml::se::to_string(&ListenResponseHeader {
					client_id: PLAYER_ID,
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
