use axum::{Extension, Json};
use sqlx::PgPool;
use tracing::{error, trace, warn};

use crate::app::{
	AppError, FriendlyRooms, ListenPlayerChannel, ServerCommandChannel, SharedPlayerState,
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
use crate::triviador::player_info::PlayerInfo;
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
		Mobile::Ping(_) => Json(MobileResponse::Ping(PingResponse::pong())),
		Mobile::MobileLogin(login_req) => {
			let mut login = LoginResponse::nopass_login(login_req.username.clone());
			if login_req.username == "felso" {
				login.data.userid = "1".to_string();
			} else {
				login.data.userid = "6".to_string();
			};
			Json(MobileResponse::Login(login))
		}
	}
}

#[axum::debug_handler]
pub async fn game(
	db: Extension<PgPool>,
	xml_header: Extension<BodyChannelType>,
	session: Extension<SharedPlayerState>,
	friendly_rooms: Extension<FriendlyRooms>,
	player_listen_channel: Extension<ListenPlayerChannel>,
	server_command_channel: Extension<ServerCommandChannel>,
	body: String,
) -> Result<String, AppError> {
	// todo fetch this from db
	const GAME_ID: u32 = 1;

	let body = format!("<ROOT>{}</ROOT>", body);
	match xml_header.0 {
		BodyChannelType::Command(comm) => {
			let ser: CommandRoot = quick_xml::de::from_str(&body)?;
			match ser.msg_type {
				CommandType::Login(login) => {
					error!(
						"Received unexpected login command in game route: {:?}",
						login
					);
					Ok(modified_xml_response(&CommandResponse::error())?)
				}
				CommandType::ChangeWaitHall(chw) => {
					trace!("Changing waithall to {:?}", chw.waithall);
					session.set_current_waithall(chw.waithall).await;

					let msg = match chw.waithall {
						Waithall::Game => quick_xml::se::to_string(&GameMenuWaithall::emulate())?,
						Waithall::Village | Waithall::Offline => {
							quick_xml::se::to_string(&VillageSetupRoot::with_name(
								session.get_player_name().await,
								session.get_player_id().await,
							))?
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
					let msg = quick_xml::se::to_string(&ExternalFriendsRoot::emulate())?;
					Ok(remove_root_tag(format!(
						"{}\n{}",
						quick_xml::se::to_string(&CommandResponse::ok(comm.client_id, comm.mn))?,
						msg
					)))
				}
				CommandType::ExitCurrentRoom(_) => {
					// todo remove player from friendly game menu
					if session.get_current_waithall().await == Waithall::Game {
						let msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
						player_listen_channel.send_message(msg).await.unwrap();
					}

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::CloseGame => {
					// send back to the menu
					let msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
					player_listen_channel.send_message(msg).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				// This gets called after clicking on the button add players to the friendly room
				CommandType::AddFriendlyRoom(request_room) => {
					trace!("{:?}", request_room);

					let room_number = friendly_rooms.0.get_next_available() as u16;

					friendly_rooms
						.insert_async(
							room_number,
							ActiveSepRoom::new(
								OpponentType::Player(session.0.get_player_id().await),
								&session.0.get_player_name().await,
							),
						)
						.await
						.unwrap();

					let mut room = friendly_rooms.0.get_async(&room_number).await.unwrap();

					room.get_mut().add_listener_player_channel(
						player_listen_channel.0.clone(),
						OpponentType::Player(session.0.get_player_id().await),
					);
					room.add_opponent(request_room.opp1, request_room.name1)
						.unwrap();
					room.add_opponent(request_room.opp2, request_room.name2)
						.unwrap();

					room.code = Some(room_number);

					room.player1_ready = true;

					room.check_playable();

					let switched_room = room.get().clone();
					switched_room.send_state_to_players().await;

					let xml = quick_xml::se::to_string(room.get())?;
					player_listen_channel.send_message(xml).await.unwrap();
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::JoinFriendlyRoom(room) => {
					if let Some(code) = room.code {
						if let Some(mut active_room) =
							friendly_rooms.0.clone().get_async(&code).await
						{
							if active_room
								.get_mut()
								.add_opponent(
									OpponentType::Player(session.get_player_id().await),
									Some(session.get_player_name().await),
								)
								.is_ok()
							{
								{
									let room_lock = active_room.get_mut();
									room_lock.add_listener_player_channel(
										player_listen_channel.0.clone(),
										OpponentType::Player(session.0.get_player_id().await),
									);
									// todo no
									room_lock.player3_ready = true;
									room_lock.check_playable();
								}
								let switched_room = active_room.get().clone();
								switched_room.send_state_to_players().await;
							} else {
								return Ok(modified_xml_response(&CommandResponse::error())?);
							}
						} else {
							warn!("Friendly room {} doesn't exist", code);
							return Ok(modified_xml_response(&CommandResponse::error())?);
						}
					} else {
						warn!("Provided code must be some");
						return Ok(modified_xml_response(&CommandResponse::error())?);
					}
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::StartTriviador(_) => {
					let info = PlayerInfo {
						p1_name: todo!(),
						p2_name: todo!(),
						p3_name: todo!(),
						pd1: todo!(),
						pd2: todo!(),
						pd3: todo!(),
						you: todo!(),
						game_id: todo!(),
						room: todo!(),
						rules: todo!(),
					};
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
					session.set_listen_ready(true).await;
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

			// todo find a better way - move this out of listen
			if session.0.get_current_waithall().await == Waithall::Offline {
				// setup village
				let msg = quick_xml::se::to_string(&VillageSetupRoot::with_name(
					session.get_player_name().await,
					session.get_player_id().await,
				))?;
				player_listen_channel.send_message(msg).await.unwrap();
				session.set_current_waithall(Waithall::Village).await;
			}

			session.set_listen_ready(ser.listen.is_ready).await;

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
