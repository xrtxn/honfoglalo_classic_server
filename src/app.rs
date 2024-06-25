use crate::api::{client_castle, mobil};
use axum::routing::{get, post};
use axum::{Extension, Router};
use sqlx::postgres::PgPool;

pub struct App {
    db: PgPool,
}

impl App {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPool::connect(&dotenvy::var("DATABASE_URL").expect("DATABASE_URL not defined!"))
            .await?;
        sqlx::migrate!().run(&db).await?;
        Ok(Self { db })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/mobil.php", post(mobil))
            .route("/client_castle.php", get(client_castle))
            .layer(Extension(self.db.clone()));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
            .await
            .unwrap();

        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }
}
