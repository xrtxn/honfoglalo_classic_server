use std::collections::VecDeque;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::{Extension, Router};
use surrealdb::engine::any;
use surrealdb::Surreal;
use tokio::sync::Mutex;

use crate::router::{client_castle, countries, friends, game, help, mobil};
use crate::triviador::TriviadorResponseRoot;

pub struct App {
	db: Surreal<any::Any>,
}

pub type SPState = Arc<Mutex<SinglePlayerState>>;
pub struct SinglePlayerState {
	pub is_listen_ready: bool,
	pub is_logged_in: bool,
	pub animation_finished: bool,
	pub triviador_state: TriviadorResponseRoot,
	pub listen_queue: VecDeque<String>,
}

impl App {
	pub async fn new() -> Result<Self, anyhow::Error> {
		// Create database connection
		let endpoint = dotenvy::var("SURREALDB_ENDPOINT").unwrap_or_else(|_| "memory".to_owned());
		let db = any::connect(endpoint).await?;
		// Select a specific namespace / database
		db.use_ns("test").use_db("test").await?;

		Ok(Self { db })
	}

	pub async fn serve(self) -> Result<(), anyhow::Error> {
		let single_player_state = SPState::new(Mutex::new(SinglePlayerState {
			is_listen_ready: false,
			is_logged_in: false,
			animation_finished: false,
			triviador_state: TriviadorResponseRoot::new_game(),
			listen_queue: VecDeque::new(),
		}));
		let app = Router::new()
			.route("/mobil.php", post(mobil))
			.route("/dat/help.json", get(help))
			.route("/game", post(game).with_state(single_player_state))
			.route("/client_countries.php", get(countries))
			.route("/client_friends.php", post(friends))
			.route("/client_castle.php", get(client_castle))
			// .route("/client_extdata.php", get(extdata))
			.layer(Extension(self.db));
		let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
		axum::serve(listener, app.into_make_service()).await?;
		Ok(())
	}
}
