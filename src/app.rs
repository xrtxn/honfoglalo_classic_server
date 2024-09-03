use axum::routing::{get, post};
use axum::{Extension, Router};
use fred::prelude::*;
use sqlx::postgres::PgPool;

use crate::router::{client_castle, countries, friends, game, help, mobil};

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
			// use exponential backoff, starting at 100 ms and doubling on each failed attempt up to
			// 30 sec
			.set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
			.build_pool(8)
			.expect("Failed to create redis pool");
		tmp_db.init().await.expect("Failed to connect to redis");
		Ok(Self { db, tmp_db })
	}

	pub async fn serve(self) -> Result<(), anyhow::Error> {
		// let single_player_state = SPState::new(Mutex::new(SinglePlayerState::new()));
		let app = Router::new()
			.route("/mobil.php", post(mobil))
			.route("/dat/help.json", get(help))
			.route("/game", post(game))
			.route("/client_countries.php", get(countries))
			.route("/client_friends.php", post(friends))
			.route("/client_castle.php", get(client_castle))
			// .route("/client_extdata.php", get(extdata))
			.layer(Extension(self.db))
			.layer(Extension(self.tmp_db));
		let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
		axum::serve(listener, app.into_make_service()).await?;
		Ok(())
	}
}
