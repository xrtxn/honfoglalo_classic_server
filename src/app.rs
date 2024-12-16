use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{middleware, Extension, Router};
use fred::prelude::*;
use http_body_util::BodyExt;
use scc::{HashMap, Queue};
use serde::Serialize;
use sqlx::postgres::PgPool;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{broadcast, mpsc};
use tracing::trace;

use crate::channels::parse_xml_multiple;
use crate::router::{client_castle, countries, friends, game, help, mobil};
use crate::users::ServerCommand;
use crate::village::start::friendly_game::ActiveSepRoom;

// #[derive(Serialize, Clone, Debug)]
// pub struct FriendlyRooms(HashMap<i32, ActiveSepRoom>);
//
// impl FriendlyRooms {
// pub fn new() -> Self {
// FriendlyRooms(HashMap::new())
// }
//
// pub fn insert(&mut self, key: i32, val: ActiveSepRoom) {
// self.0.insert(key, val).unwrap();
// }
//
// pub fn get(&self, key: &i32) {
// self.0.get(key).unwrap();
// }
// }

pub type SharedState = HashMap<i32, SharedPlayerState>;

#[derive(Debug)]
struct PlayerState {
	pub is_logged_in: RwLock<bool>,
	pub is_listen_ready: RwLock<bool>,
	pub server_command: RwLock<Option<ServerCommand>>,
	pub listen_queue: Queue<String>,
}

#[derive(Clone, Debug)]
pub struct SharedPlayerState(Arc<PlayerState>);

impl SharedPlayerState {
	fn new() -> Self {
		let val = PlayerState {
			is_logged_in: RwLock::new(false),
			is_listen_ready: RwLock::new(false),
			server_command: RwLock::new(None),
			listen_queue: Queue::default(),
		};
		SharedPlayerState(Arc::new(val))
	}

	pub async fn get_login(&self) -> bool {
		*self.0.is_logged_in.read().unwrap()
	}

	pub async fn get_listen_ready(&self) -> bool {
		*self.0.is_listen_ready.read().unwrap()
	}

	pub async fn get_server_command(&self) -> Option<ServerCommand> {
		self.0.server_command.read().unwrap().clone()
	}

	pub async fn get_next_listen(&self) -> Option<String> {
		self.0.listen_queue.pop().map(|x| x.to_string())
	}

	pub async fn set_login(&self, val: bool) {
		*self.0.is_logged_in.write().unwrap() = val;
	}
	pub async fn set_listen_ready(&self, val: bool) {
		*self.0.is_listen_ready.write().unwrap() = val;
	}
	pub async fn set_server_command(&self, val: Option<ServerCommand>) {
		*self.0.server_command.write().unwrap() = val;
	}
	pub async fn push_listen(&self, msg: String) {
		self.0.listen_queue.push(msg);
	}
}

#[derive(Debug)]
struct PlayerChannel<T: Clone> {
	tx: broadcast::Sender<T>,
	// rx: broadcast::Receiver<T>,
}

impl<T: Clone> PlayerChannel<T> {
	fn new() -> Self {
		let (tx, rx) = broadcast::channel(8);
		PlayerChannel { tx }
	}
}

#[derive(Clone)]
pub struct SharedPlayerChannel<T: Clone>(Arc<RwLock<PlayerChannel<T>>>);

impl<T: Clone> SharedPlayerChannel<T> {
	pub fn new() -> Self {
		SharedPlayerChannel(Arc::new(RwLock::new(PlayerChannel::new())))
	}

	pub async fn new_receiver(&self) -> Receiver<T> {
	   self.0.read().unwrap().tx.subscribe()
	}

	pub async fn send_message(&self, msg: T) -> Result<usize, broadcast::error::SendError<T>> {
		self.0.read().unwrap().tx.send(msg)
	}

	pub async fn recv_message(mut receiver: Receiver<T>) -> Result<T, broadcast::error::RecvError> {
		receiver.recv().await
	}

	pub async fn is_user_listening(&self) -> bool {
		self.0.read().unwrap().tx.receiver_count() >= 1
	}
}

pub struct App {
	db: PgPool,
	tmp_db: RedisPool,
}

impl App {
	pub async fn new() -> Result<Self, anyhow::Error> {
		let db = PgPool::connect(&dotenvy::var("DATABASE_URL").expect("DATABASE_URL not defined!"))
			.await?;
		sqlx::migrate!().run(&db).await?;
		let config =
			RedisConfig::from_url(&dotenvy::var("TMP_DB_URL").expect("TMP_DB_URL not defined!"))
				.expect("Failed to create redis config from url");
		let tmp_db = Builder::from_config(config)
			.with_connection_config(|config| {
				config.connection_timeout = std::time::Duration::from_secs(10);
			})
			// use exponential backoff, starting at 100 ms and doubling on each failed attempt
			// up to 30 sec
			.set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
			.build_pool(8)
			.expect("Failed to create redis pool");
		tmp_db.init().await.expect("Failed to connect to redis");
		Ok(Self { db, tmp_db })
	}

	pub async fn serve(self) -> Result<(), AppError> {
		// todo use a middleware instead
		const USER_ID: i32 = 1;

		// let friendly_rooms: FriendlyRooms = FriendlyRooms(HashMap::new());
		let shared_state: SharedState = HashMap::new();
		let val = SharedPlayerState::new();
		let player_channel: SharedPlayerChannel<String> = SharedPlayerChannel::new();

		shared_state.insert(USER_ID, val).unwrap();

		let app = Router::new()
			.route("/mobil.php", post(mobil))
			.route("/dat/help.json", get(help))
			.route("/client_countries.php", get(countries))
			.route("/client_friends.php", post(friends))
			.route("/client_castle.php", get(client_castle))
			// .route("/client_extdata.php", get(extdata))
			.layer(Extension(self.db.clone()))
			.layer(Extension(self.tmp_db.clone()));

		let user_state = shared_state.get(&USER_ID).unwrap().get().clone();

		let game_router = Router::new()
			.route("/game", post(game))
			.route_layer(middleware::from_fn(xml_header_extractor))
			.layer(Extension(self.db.clone()))
			.layer(Extension(self.tmp_db.clone()))
			.layer(Extension(user_state.clone()))
			// .layer(Extension(friendly_rooms))
			.layer(Extension(player_channel.clone()));

		let merged = app.merge(game_router);

		let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
		axum::serve(listener, merged.into_make_service())
			.await
			.map_err(|e| AppError::from(e))?;
		Ok(())
	}
}

async fn xml_header_extractor(request: Request, next: Next) -> Response {
    println!("middleware called");
	let (parts, body) = request.into_parts();
	let bytes = body
		.collect()
		.await
		.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
		.unwrap()
		.to_bytes();

	let body = String::from_utf8_lossy(&bytes).to_string();
	let mut lines: Vec<&str> = body.lines().collect();
	println!("lines: {:?}", lines);
	let xml_header_string = lines.remove(0);
	let new_body = lines.get(0).unwrap().to_string();
	let mut req = Request::from_parts(parts, Body::from(new_body));

	let parsed_header = parse_xml_multiple(&xml_header_string).unwrap();
	req.extensions_mut().insert(parsed_header);

	let response = next.run(req).await;
	response
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

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
	E: Into<anyhow::Error>,
{
	fn from(err: E) -> Self {
		Self(err.into())
	}
}
