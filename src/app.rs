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
use tracing::trace;

use crate::channels::parse_xml_multiple;
use crate::router::{client_castle, countries, friends, game, help, mobil};
use crate::users::ServerCommand;
use crate::village::start::friendly_game::{ActiveSepRoom, OpponentType};
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

pub type SharedState = HashMap<i32, SharedPlayerState>;

#[derive(Debug)]
struct PlayerState {
	is_logged_in: RwLock<bool>,
	is_listen_ready: RwLock<bool>,
	current_waithall: RwLock<Waithall>,
	player_id: RwLock<i16>,
	player_name: String,
}

#[derive(Clone, Debug)]
pub struct SharedPlayerState(Arc<PlayerState>);

impl SharedPlayerState {
	fn new() -> Self {
		let val = PlayerState {
			is_logged_in: RwLock::new(false),
			is_listen_ready: RwLock::new(false),
			current_waithall: RwLock::new(Waithall::Village),
			player_id: RwLock::new(0),
			player_name: "Anonymous".to_string(),
		};
		SharedPlayerState(Arc::new(val))
	}

	#[allow(dead_code)]
	pub async fn get_listen_ready(&self) -> bool {
		*self.0.is_listen_ready.read().await
	}
	pub async fn set_login(&self, val: bool) {
		*self.0.is_logged_in.write().await = val;
	}
	pub async fn set_listen_ready(&self, val: bool) {
		*self.0.is_listen_ready.write().await = val;
	}
	pub async fn get_current_waithall(&self) -> Waithall {
		*self.0.current_waithall.read().await
	}
	pub async fn set_current_waithall(&self, waithall: Waithall) {
		*self.0.current_waithall.write().await = waithall;
	}
	pub async fn get_player_id(&self) -> i16 {
		*self.0.player_id.read().await
	}
	pub async fn set_player_id(&self, player_id: i16) {
		*self.0.player_id.write().await = player_id;
	}
	pub fn get_player_name(&self) -> String {
		self.0.player_name.clone()
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
		let session_store = tower_sessions::MemoryStore::default();
		let session_layer =
			tower_sessions::SessionManagerLayer::new(session_store).with_secure(false);

		trace!("Starting server on port 8080");
		let friendly_rooms: FriendlyRooms = FriendlyRooms::new();
		let player_state = SharedPlayerState::new();
		// todo fix one channel for everyone
		let player_channel: XmlPlayerChannel = PlayerChannel::new();
		let server_command_channel: ServerCommandChannel = ServerCommandChannel::new();

		let mut test_room = ActiveSepRoom::new(OpponentType::Player(2), "Tesztelek");
		test_room.add_opponent(OpponentType::Robot, None).unwrap();
		test_room.add_opponent(OpponentType::Robot, None).unwrap();
		friendly_rooms.insert_async(0000, test_room).await.unwrap();

		let app = Router::new()
			.route("/mobil.php", post(mobil))
			.route("/dat/help.json", get(help))
			.route("/client_countries.php", get(countries))
			.route("/client_friends.php", post(friends))
			.route("/client_castle.php", get(client_castle));
		// .route("/client_extdata.php", get(extdata));

		let game_router = Router::new()
			.route("/game", post(game))
			.route_layer(middleware::from_fn(xml_header_extractor))
			.layer(Extension(self.db.clone()))
			.layer(Extension(player_state))
			.layer(Extension(friendly_rooms))
			.layer(Extension(player_channel))
			.layer(Extension(server_command_channel))
			.layer(session_layer);

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

		let parsed_header = parse_xml_multiple(xml_header_string).unwrap();
		req.extensions_mut().insert(parsed_header);
		req
	};

	next.run(req).await
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
