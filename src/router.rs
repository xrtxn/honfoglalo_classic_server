use std::collections::HashMap;

use anyhow::anyhow;
use axum::extract::Query;
use axum::{Extension, Json};
use fred::clients::RedisPool;
use fred::prelude::*;
use sqlx::PgPool;
use tracing::warn;

use crate::app::AppError;
use crate::cdn::countries::CountriesResponse;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseType::VillageSetup;
use crate::channels::listen::response::{ListenResponse, ListenResponseHeader};
use crate::channels::{ChannelErrorResponse, ChannelType};
use crate::emulator::Emulator;
use crate::game_handler::server_game_handler::ServerGameHandler;
use crate::game_handler::wait_for_game_ready;
use crate::menu::friend_list::external_data::ExternalFriendsRoot;
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::mobile::request::Mobile;
use crate::mobile::response::{LoginResponse, MobileResponse, PingResponse};
use crate::users::{ServerCommand, User};
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
	_db: Extension<PgPool>,
	tmp_db: Extension<RedisPool>,
	headers: Query<HashMap<String, String>>,
	body: String,
) -> Result<String, AppError> {
	const GAME_ID: u32 = 1;
	const PLAYER_ID: i32 = 1;
	const PLAYER_NAME: &str = "xrtxn";
	let lines: Vec<&str> = body.lines().collect();

	let string = format!(
		"<ROOT>{}</ROOT>",
		match lines.get(1) {
			None => {
				return Err(AppError::from(anyhow!(
					"Invalid body xml, possibly without header"
				)));
			}
			Some(r) => {
				r
			}
		}
	);
	let headers = {
		let serialized_headers = serde_json::to_string(&headers.0)?;
		serde_json::from_str::<ChannelType>(&serialized_headers)?
	};
	match headers {
		ChannelType::Command(comm) => {
			let ser: CommandRoot = quick_xml::de::from_str(&string)?;
			match ser.msg_type {
				CommandType::Login(_) => {
					// todo validate login
					User::reset(&tmp_db, PLAYER_ID).await?;
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
							msg = quick_xml::se::to_string(&VillageSetupRoot::emulate())?
						}
					}
					User::push_listen_queue(&tmp_db, PLAYER_ID, &msg).await?;
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
						let room_number = ActiveSepRoom::get_next_num(&tmp_db).await?;
						ActiveSepRoom::new_bots_room(&tmp_db, room_number, PLAYER_ID, PLAYER_NAME)
							.await?;
						let xml = ActiveSepRoom::get_active(&tmp_db, room_number).await?;
						User::push_listen_queue(
							&tmp_db,
							PLAYER_ID,
							&quick_xml::se::to_string(&xml)?,
						)
						.await?;
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
					User::set_listen_state(&tmp_db, PLAYER_ID, true).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::SelectArea(area_selection) => {
					User::set_server_command(
						&tmp_db,
						PLAYER_ID,
						ServerCommand::SelectArea(area_selection.area),
					)
					.await?;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::QuestionAnswer(ans) => {
					User::set_server_command(
						&tmp_db,
						PLAYER_ID,
						ServerCommand::QuestionAnswer(ans.get_answer()),
					)
					.await?;

					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
			}
		}
		ChannelType::Listen(lis) => {
			let ser: ListenRoot = quick_xml::de::from_str(&string)?;
			User::set_listen_state(&tmp_db, PLAYER_ID, ser.listen.is_ready).await?;
			if !User::get_is_logged_in(&tmp_db, PLAYER_ID).await? {
				User::set_is_logged_in(&tmp_db, PLAYER_ID, true).await?;
				return Ok(modified_xml_response(&ListenResponse::new(
					ListenResponseHeader {
						client_id: lis.client_id,
						mn: lis.mn,
						result: 0,
					},
					VillageSetup(VillageSetupRoot::emulate()),
				))?);
			}

			if !User::is_listen_empty(&tmp_db, PLAYER_ID).await? {
				if !User::get_listen_state(&tmp_db, PLAYER_ID).await? {
					wait_for_game_ready(&tmp_db, PLAYER_ID).await;
				}
				let next_listen = match User::pop_listen_queue(&tmp_db, PLAYER_ID).await {
					None => {
						return Err(AppError::from(anyhow!(
							"Invalid body xml, possibly without header"
						)));
					}
					Some(res) => res,
				};
				return Ok(format!(
					"{}\n{}",
					quick_xml::se::to_string(&ListenResponseHeader {
						client_id: PLAYER_ID.to_string(),
						mn: lis.mn,
						result: 0,
					})?,
					remove_root_tag(next_listen)
				));
			}

			let subscriber = Builder::default_centralized().build()?;
			subscriber.init().await?;
			subscriber
				.psubscribe(format!("__key*__:users:{}:listen_queue", PLAYER_ID))
				.await?;
			let mut keyspace_rx = subscriber.keyspace_event_rx();

			let event = keyspace_rx.recv().await?;
			if event.operation == "rpush" {
				User::set_listen_state(&tmp_db, PLAYER_ID, false).await?;
				let next_listen = match User::pop_listen_queue(&tmp_db, PLAYER_ID).await {
					None => {
						return Err(AppError::from(anyhow!(
							"Invalid body xml, possibly without header"
						)));
					}
					Some(res) => res,
				};
				return Ok(format!(
					"{}\n{}",
					quick_xml::se::to_string(&ListenResponseHeader {
						client_id: PLAYER_ID.to_string(),
						mn: lis.mn,
						result: 0,
					})?,
					remove_root_tag(next_listen)
				));
			}
			// this theoretically never happens but this makes the compiler happy
			Ok(modified_xml_response(&ChannelErrorResponse {})?)
		}
	}
}

pub async fn client_castle() -> Json<CastleResponse> {
	Json(CastleResponse::emulate())
}
