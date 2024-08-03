use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use axum::extract::Query;
use axum::{extract, Extension, Json};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::app::SinglePlayerState;
use crate::constant_setup::countries::response::CountriesResponse;
use crate::emulator::{remove_root_tag, Emulator};
use crate::game::request::{ChannelType, CommandRoot, CommandType};
use crate::game::response::CommandResponse;
use crate::login_screen::request::Mobile;
use crate::login_screen::response::{LoginResponse, MobileResponse, PingResponse};
use crate::menu::friend_list::friends::FriendResponse;
use crate::menu::help::info_help::HelpResponse;
use crate::village::castle::badges::CastleResponse;
use crate::village::setup::VillageSetupRoot;
use crate::village::waithall::response::GameMenuWaithall;

pub async fn help(_pool: Extension<PgPool>) -> Json<HelpResponse> {
    //todo find out how to use this
    Json(HelpResponse::emulate("".to_string()))
}

pub async fn countries(_pool: Extension<PgPool>) -> Json<CountriesResponse> {
    Json(CountriesResponse::emulate("".to_string()))
}
pub async fn friends(_pool: Extension<PgPool>) -> Json<FriendResponse> {
    Json(FriendResponse::emulate("".to_string()))
}

pub async fn mobil(_pool: Extension<PgPool>, Json(payload): Json<Mobile>) -> Json<MobileResponse> {
    match payload {
        Mobile::Ping(_) => {
            return Json(MobileResponse::Ping(PingResponse {
                message: "pong".to_string(),
            }))
        }
        Mobile::Login(_) => Json(MobileResponse::Login(LoginResponse::emulate(
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
                    //todo match chw
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
                    return vec.listen_queue.pop_front().unwrap();
                }
                drop(vec);

                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        }
    }
}

#[axum::debug_handler]
pub async fn client_castle(_pool: Extension<PgPool>) -> Json<CastleResponse> {
    Json(CastleResponse::emulate("".to_string()))
}
