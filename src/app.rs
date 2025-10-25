use std::hash::RandomState;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Router, middleware};
use http_body_util::BodyExt;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use scc::HashMap;
use scc::hash_map::OccupiedEntry;
use sqlx::postgres::PgPool;
use tokio::sync::RwLock;
use tracing::{error, trace, warn};

use crate::channels::command::request::CommandRoot;
use crate::channels::{BodyChannelType, parse_xml_multiple};
use crate::router::{client_castle, countries, friends, game, help, mobil};
use crate::users::ServerCommand;
use crate::village::start::friendly_game::ActiveSepRoom;
use crate::village::waithall::Waithall;

#[derive(Clone)]
pub struct FriendlyRooms(pub std::sync::Arc<HashMap<u16, ActiveSepRoom>>);

impl FriendlyRooms {
	pub fn new() -> Self {
		FriendlyRooms(std::sync::Arc::new(HashMap::new()))
	}

	pub async fn insert_async(
		&self,
		key: u16,
		value: ActiveSepRoom,
	) -> Result<(), (u16, ActiveSepRoom)> {
		self.0.insert_async(key, value).await
	}

	// todo check this out as this has weird behavior
	pub async fn get_async(
		&self,
		key: &u16,
	) -> Option<OccupiedEntry<'_, u16, ActiveSepRoom, RandomState>> {
		self.0.get_async(key).await
	}

	#[allow(dead_code)]
	pub async fn remove_async(&self, key: &u16) -> Option<ActiveSepRoom> {
		self.0.remove_async(key).await.map(|(_, v)| v)
	}

	pub fn get_next_available(&self) -> usize {
		let mut rng = StdRng::from_entropy();
		let num = rng.gen_range(1000..=9999);
		trace!("Generated friendly room code: {}", num);
		num
	}
}

type SharedState = Arc<HashMap<i32, SharedPlayerState>>;

#[derive(Debug)]
pub(crate) struct PlayerState {
	is_logged_in: bool,
	is_listen_ready: bool,
	current_waithall: Waithall,
	player_id: i32,
	player_name: String,
	command_channel: ServerCommandChannel,
	listen_channel: XmlPlayerChannel,
}

#[derive(Clone, Debug)]
pub struct SharedPlayerState(pub Arc<RwLock<PlayerState>>);

impl SharedPlayerState {
	fn new() -> Self {
		let val = PlayerState {
			is_logged_in: false,
			is_listen_ready: false,
			current_waithall: Waithall::Offline,
			player_id: 0,
			player_name: "Anonymous".to_string(),
			command_channel: ServerCommandChannel::new(),
			listen_channel: XmlPlayerChannel::new(),
		};
		SharedPlayerState(Arc::new(RwLock::new(val)))
	}

	#[allow(dead_code)]
	pub async fn get_listen_ready(&self) -> bool {
		self.0.read().await.is_listen_ready
	}
	pub async fn set_login(&self, val: bool) {
		self.0.write().await.is_logged_in = val;
	}
	pub async fn set_listen_ready(&self, val: bool) {
		self.0.write().await.is_listen_ready = val;
	}
	pub async fn get_current_waithall(&self) -> Waithall {
		self.0.read().await.current_waithall.clone()
	}
	pub async fn set_current_waithall(&self, waithall: Waithall) {
		self.0.write().await.current_waithall = waithall;
	}
	pub async fn get_player_id(&self) -> i32 {
		self.0.read().await.player_id
	}
	pub async fn get_player_name(&self) -> String {
		self.0.read().await.player_name.clone()
	}
}

#[derive(Clone, Debug)]
pub struct PlayerChannel<T: Clone> {
	tx: flume::Sender<T>,
	rx: flume::Receiver<T>,
}

impl<T: Clone> PlayerChannel<T> {
	fn new() -> Self {
		let (tx, rx) = flume::bounded(8);
		PlayerChannel { tx, rx }
	}

	pub(crate) async fn send_message(&self, msg: T) -> Result<(), flume::SendError<T>> {
		self.tx.send_async(msg).await
	}

	pub(crate) async fn recv_message(&self) -> Result<T, flume::RecvError> {
		self.rx.recv_async().await
	}

	pub(crate) fn clear_rx(&self) {
		let num = self.rx.len();
		while self.rx.try_recv().is_ok() {
			trace!("Clearing message from channel: {}", num);
		}
	}
}

pub type ServerCommandChannel = PlayerChannel<ServerCommand>;
pub type XmlPlayerChannel = PlayerChannel<String>;

pub struct App {
	db: PgPool,
}

impl App {
	pub async fn new() -> Result<Self, anyhow::Error> {
		let db = PgPool::connect(&dotenvy::var("DATABASE_URL").expect("DATABASE_URL not defined!"))
			.await?;
		sqlx::migrate!().run(&db).await?;
		Ok(Self { db })
	}

	pub async fn serve(self) -> Result<(), AppError> {
		trace!("Starting server on port 8080");
		let friendly_rooms: FriendlyRooms = FriendlyRooms::new();
		let shared_state: SharedState = Arc::new(HashMap::new());

		let app = Router::new()
			.route("/mobil.php", post(mobil))
			.route("/dat/help.json", get(help))
			.route("/client_countries.php", get(countries))
			.route("/client_friends.php", post(friends))
			.route("/client_castle.php", get(client_castle))
			.layer(Extension(self.db.clone()));
		// .route("/client_extdata.php", get(extdata));

		let game_router = Router::new()
			.route("/game", post(game))
			.route_layer(middleware::from_fn_with_state(
				shared_state.clone(),
				set_session_for_player,
			))
			.route_layer(middleware::from_fn(auth))
			.route_layer(middleware::from_fn(xml_header_extractor))
			.layer(Extension(self.db.clone()))
			.layer(Extension(friendly_rooms));

		let merged = app.merge(game_router);

		let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
		axum::serve(listener, merged.into_make_service())
			.await
			.map_err(AppError::from)?;
		Ok(())
	}
}

async fn xml_header_extractor(request: Request, next: Next) -> Response {
	let req = {
		let (parts, body) = request.into_parts();
		let bytes = body
			.collect()
			.await
			.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
			.unwrap()
			.to_bytes();

		let body = String::from_utf8_lossy(&bytes).to_string();
		let mut lines: Vec<&str> = body.lines().collect();
		let xml_header_string = lines.remove(0);
		// necessary else for heartbeat requests
		let new_body = if !lines.is_empty() {
			lines.remove(0)
		} else {
			""
		};
		let mut req = Request::from_parts(parts, Body::from(new_body.to_string()));

		let parsed_header: BodyChannelType = parse_xml_multiple(xml_header_string).unwrap();
		req.extensions_mut().insert(parsed_header);
		req
	};

	next.run(req).await
}

async fn auth(request: Request, next: Next) -> Response {
	use crate::channels::NO_CID;

	// get cid from above parsed xml header
	if let Some(ax) = request
		.extensions()
		.clone()
		.get::<crate::channels::BodyChannelType>()
	{
		match ax {
			crate::channels::BodyChannelType::Command(cmd) => {
				if cmd.client_id == NO_CID {
					let new_cid = {
						let body = request.into_body().collect().await.unwrap().to_bytes();
						let body_str = String::from_utf8_lossy(&body).to_string();
						let ser: CommandRoot =
							quick_xml::de::from_str(&format!("<ROOT>{}</ROOT>", body_str)).unwrap();
						match ser.msg_type {
							crate::channels::command::request::CommandType::Login(login) => {
								if login.name == "a" { 1 } else { 6 }
							}
							_ => {
								error!("Unauthorized command with NO_CID: {:?}", cmd);
								return StatusCode::UNAUTHORIZED.into_response();
							}
						}
					};

					let resp = crate::utils::modified_xml_response(
						&crate::channels::command::response::CommandResponse::ok(new_cid, cmd.mn),
					)
					.unwrap();
					resp.into_response()
				} else {
					next.run(request).await
				}
			}
			_ => next.run(request).await,
		}
	} else {
		warn!("BodyChannelType is none!");
		StatusCode::UNAUTHORIZED.into_response()
	}
}

async fn set_session_for_player(
	axum::extract::State(state): axum::extract::State<SharedState>,
	mut request: Request,
	next: Next,
) -> Response {
	trace!(
		"Request reached set_session_for_player middleware {:?}",
		request.uri()
	);
	// get cid from above parsed xml header
	let cid = if let Some(ax) = request
		.extensions()
		.clone()
		.get::<crate::channels::BodyChannelType>()
	{
		match ax {
			crate::channels::BodyChannelType::Command(cmd) => cmd.client_id,
			crate::channels::BodyChannelType::Listen(lis) => lis.client_id,
			crate::channels::BodyChannelType::HeartBeat(hb) => hb.client_id,
		}
	} else {
		warn!("No cid found for hashmap!");
		-128
	};

	let player_state = {
		let pstate = state
			.entry_async(cid)
			.await
			.or_insert_with(SharedPlayerState::new);

		pstate.get().clone()
	};

	player_state.0.write().await.player_id = cid;

	request
		.extensions_mut()
		.insert(player_state.0.read().await.listen_channel.clone());
	request
		.extensions_mut()
		.insert(player_state.0.read().await.command_channel.clone());
	// set session for player based on cid
	request.extensions_mut().insert(player_state.clone());

	next.run(request).await
}

#[derive(Debug)]
pub(crate) struct AppError(anyhow::Error);

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {}", self.0),
		)
			.into_response()
	}
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>`
// to turn them into `Result<_, AppError>`.
impl<E> From<E> for AppError
where
	E: Into<anyhow::Error>,
{
	fn from(err: E) -> Self {
		Self(err.into())
	}
}
