use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::app::{App, AppError};

mod app;
mod cdn;
mod channels;
mod emulator;
mod game_handlers;
mod login_screen;
mod menu;
mod mobile;
mod router;
mod triviador;
mod users;
mod utils;
mod village;

#[tokio::main]
async fn main() -> Result<(), AppError> {
	tracing_subscriber::registry()
		.with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
			|_| "sqlx=warn,tower_http=debug,honfoglalo_classic_server=trace,fred=info".into(),
		)))
		.with(tracing_subscriber::fmt::layer())
		.init();

	App::new().await?.serve().await
}
