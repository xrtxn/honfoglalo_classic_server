use axum::{Extension, Json};
use sqlx::PgPool;

use crate::request_structs::Mobile;
use crate::response_structs::{Badges, CastleResponse, NewBadgeLevels, PingResponse};

pub async fn mobil(
	pool: Extension<PgPool>,
	Json(payload): Json<Mobile>,
) -> Json<PingResponse> {
	Json(PingResponse {
		message: "pong".to_string(),
	})
}


pub async fn client_castle(
	pool: Extension<PgPool>,
) -> Json<CastleResponse> {
	Json(CastleResponse {
		error: "0".to_string(),
		data: Badges { castle_badges: vec![], other_badges: vec![] },
		new_levels: NewBadgeLevels { vec: vec![] },
	})
}
