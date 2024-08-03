// use crate::login_screen::response::CastleResponse;
// use crate::village::castle::badges::{all_badges, all_castle_badges, Badges};
// use axum::{Extension, Json};
// use sqlx::PgPool;
//
// pub async fn client_castle(pool: Extension<PgPool>) -> Json<CastleResponse> {
//     Json(CastleResponse {
//         error: "0".to_string(),
//         data: Badges {
//             castle_badges: all_castle_badges(),
//             new_levels: all_badges(),
//         },
//     })
// }
