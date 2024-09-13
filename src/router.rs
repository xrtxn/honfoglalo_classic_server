use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail};
use axum::extract::Query;
use axum::{Extension, Json};
use fred::clients::RedisPool;
use sqlx::PgPool;
use tokio::try_join;
use tracing::warn;

use crate::app::AppError;
use crate::cdn::countries::CountriesResponse;
use crate::channels::command::request::{CommandRoot, CommandType};
use crate::channels::command::response::CommandResponse;
use crate::channels::listen::request::ListenRoot;
use crate::channels::listen::response::ListenResponseType::VillageSetup;
use crate::channels::listen::response::{ListenResponse, ListenResponseHeader};
use crate::channels::ChannelType;
use crate::emulator::Emulator;
use crate::menu::friend_list::external_data::ExternalFriendsRoot;
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::mobile::request::Mobile;
use crate::mobile::response::{LoginResponse, MobileResponse, PingResponse};
use crate::triviador::{AvailableAreas, GameState, TriviadorGame};
use crate::users;
use crate::utils::{modified_xml_response, remove_root_tag};
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
					users::User::reset(&tmp_db, PLAYER_ID).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						PLAYER_ID, comm.mn,
					))?)
				}
				CommandType::ChangeWaitHall(_) => {
					let msg = quick_xml::se::to_string(&GameMenuWaithall::emulate())?;
					users::User::push_listen_queue(&tmp_db, PLAYER_ID, &msg).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::EnterGameLobby(_) => {
					todo!()
					// state.0.lock().await.listen_queue.push_back(xml);
					// quick_xml::se::to_string(&CommandResponse::ok(comm.
					// client_id, comm.mn)).unwrap()
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
				CommandType::AddFriendlyRoom(_) => {
					let xml = quick_xml::se::to_string(&ActiveSepRoom::new_bots_room(
						PLAYER_ID,
						PLAYER_NAME,
					))?;
					users::User::push_listen_queue(&tmp_db, PLAYER_ID, &xml).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::StartTriviador(_) => {
					// todo make the server handle the game

					let xml = quick_xml::se::to_string(
						&TriviadorGame::new_game(&tmp_db, GAME_ID).await?,
					)?;
					users::User::push_listen_queue(&tmp_db, PLAYER_ID, &xml).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::PlayerReady => {
					if users::User::is_listen_ready(&tmp_db, PLAYER_ID).await? {
						tokio::time::sleep(Duration::from_millis(2000)).await;
						return Ok(modified_xml_response(&CommandResponse::ok(
							comm.client_id,
							comm.mn,
						))?);
					}
					let a_listen = users::User::set_listen_ready(&tmp_db, PLAYER_ID, true);
					let gamestate = GameState::get_gamestate(&tmp_db, GAME_ID);
					let gamestate = match try_join!(a_listen, gamestate) {
						Ok(g) => g.1,
						Err(e) => return Err(AppError::from(anyhow!(e))),
					};

					match gamestate.state {
						11 => {
							TriviadorGame::announcement(&tmp_db, GAME_ID).await?;
						}
						1 => match gamestate.phase {
							0 => {
								TriviadorGame::choose_area(&tmp_db, GAME_ID).await?;
							}
							1 => {
								return Ok(modified_xml_response(&CommandResponse::ok(
									comm.client_id,
									comm.mn,
								))?);
							}
							_ => {
								todo!()
							}
						},
						_ => {
							todo!()
						}
					}
					let xml = quick_xml::se::to_string(
						&TriviadorGame::get_triviador(&tmp_db, GAME_ID).await?,
					)?;
					users::User::push_listen_queue(&tmp_db, PLAYER_ID, &xml).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
				CommandType::SelectArea(area_selection) => {
					AvailableAreas::pop_county(&tmp_db, GAME_ID, area_selection.area.try_into()?)
						.await?;

					GameState::set_gamestate(
						&tmp_db,
						GAME_ID,
						GameState {
							state: 1,
							gameround: 0,
							phase: 2,
						},
					)
					.await?;
					let xml = quick_xml::se::to_string(
						&TriviadorGame::get_triviador(&tmp_db, GAME_ID).await?,
					)?;
					users::User::push_listen_queue(&tmp_db, PLAYER_ID, &xml).await?;
					Ok(modified_xml_response(&CommandResponse::ok(
						comm.client_id,
						comm.mn,
					))?)
				}
			}
		}
		ChannelType::Listen(lis) => {
			let ser: ListenRoot = quick_xml::de::from_str(&string)?;
			users::User::set_listen_ready(&tmp_db, PLAYER_ID, ser.listen.is_ready).await?;
			if !ser.listen.is_ready {
				while !users::User::is_listen_ready(&tmp_db, PLAYER_ID).await? {
					tokio::time::sleep(Duration::from_millis(1000)).await;
				}
			}
			if !users::User::get_is_logged_in(&tmp_db, PLAYER_ID).await? {
				users::User::set_is_logged_in(&tmp_db, PLAYER_ID, true).await?;
				return Ok(modified_xml_response(&ListenResponse::new(
					ListenResponseHeader {
						client_id: lis.client_id,
						mn: lis.mn,
						result: 0,
					},
					VillageSetup(VillageSetupRoot::emulate()),
				))?);
			}
			loop {
				if !users::User::is_listen_empty(&tmp_db, PLAYER_ID).await? {
					if lis.mn == "2" {
						let mut t = ActiveSepRoom::new_bots_room(PLAYER_ID, PLAYER_NAME);
						t.start_friendly_room();
						let msg = quick_xml::se::to_string(&t)?;
						users::User::push_listen_queue(&tmp_db, 1, &msg).await?;
						tokio::time::sleep(Duration::from_millis(1000)).await;
					}

					let next_listen = match users::User::get_next_listen(&tmp_db, PLAYER_ID).await {
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

				tokio::time::sleep(Duration::from_millis(1000)).await;
			}
		}
	}
}

pub async fn client_castle() -> Json<CastleResponse> {
	Json(CastleResponse::emulate())
}
