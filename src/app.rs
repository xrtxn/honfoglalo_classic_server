use crate::api::{client_castle, countries, friends, game, help, mobil};
use axum::routing::{get, post};
use axum::{Extension, Router};
use sqlx::postgres::PgPool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct App {
    db: PgPool,
}

pub type SPState = Arc<Mutex<SinglePlayerState>>;
pub struct SinglePlayerState {
    pub is_logged_in: bool,
    pub listen_queue: VecDeque<String>,
}

impl App {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPool::connect(&dotenvy::var("DATABASE_URL").expect("DATABASE_URL not defined!"))
            .await?;
        sqlx::migrate!().run(&db).await?;
        Ok(Self { db })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let single_player_state = SPState::new(Mutex::new(SinglePlayerState {
            is_logged_in: false,
            listen_queue: VecDeque::new(),
        }));
        let app = Router::new()
            .route("/mobil.php", post(mobil))
            .route("/dat/help.json", get(help))
            .route("/game", post(game).with_state(single_player_state))
            .route("/client_countries.php", get(countries))
            .route("/client_friends.php", post(friends))
            .route("/client_castle.php", get(client_castle))
            .layer(Extension(self.db.clone()));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }
}
