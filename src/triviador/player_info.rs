use std::collections::HashMap;

use anyhow::bail;
use fred::clients::RedisPool;
use fred::prelude::*;
use futures::TryFutureExt;
use serde::Serialize;
use tokio::try_join;

use crate::triviador::game_player_data::GamePlayerData;

#[derive(Serialize, Debug, Clone)]
pub struct PlayerInfo {
	#[serde(rename = "@P1")]
	pub p1_name: String,
	#[serde(rename = "@P2")]
	pub p2_name: String,
	#[serde(rename = "@P3")]
	pub p3_name: String,
	#[serde(rename = "@PD1")]
	pub pd1: GamePlayerData,
	#[serde(rename = "@PD2")]
	pub pd2: GamePlayerData,
	#[serde(rename = "@PD3")]
	pub pd3: GamePlayerData,
	#[serde(rename = "@YOU")]
	pub you: String,
	#[serde(rename = "@GAMEID")]
	pub game_id: String,
	#[serde(rename = "@ROOM")]
	pub room: String,
	#[serde(rename = "@RULES")]
	// 1,0 possibly means quick game
	pub rules: String,
}

impl PlayerInfo {
	pub async fn set_info(
		tmppool: &RedisPool,
		game_id: u32,
		info: PlayerInfo,
	) -> Result<u8, anyhow::Error> {
		{
			let gpd_one_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 1, info.pd1);
			let gpd_two_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 2, info.pd2);
			let gpd_three_fut = GamePlayerData::set_game_player_data(tmppool, game_id, 3, info.pd3);
			let info_fut = tmppool.hset::<u8, _, _>(
				format!("games:{}:info", game_id),
				[
					("p1_name", info.p1_name),
					("p2_name", info.p2_name),
					("p3_name", info.p3_name),
					("you", info.you),
					("game_id", info.game_id),
					("room", info.room),
					("rules", info.rules),
				],
			);
			let mut modified = 0;
			let res = try_join!(
				gpd_one_fut,
				gpd_two_fut,
				gpd_three_fut,
				info_fut.map_err(anyhow::Error::from)
			);
			match res {
				Ok(res) => {
					modified += res.0;
					modified += res.1;
					modified += res.2;
					modified += res.3;
				}
				Err(err) => bail!(err),
			}
			Ok(modified)
		}
	}
	pub async fn get_info(tmppool: &RedisPool, game_id: u32) -> Result<PlayerInfo, anyhow::Error> {
		let res: HashMap<String, String> =
			tmppool.hgetall(format!("games:{}:info", game_id)).await?;
		let pd1 = GamePlayerData::get_game_player_data(tmppool, game_id, 1).await?;
		let pd2 = GamePlayerData::get_game_player_data(tmppool, game_id, 2).await?;
		let pd3 = GamePlayerData::get_game_player_data(tmppool, game_id, 3).await?;
		Ok(PlayerInfo {
			p1_name: res.get("p1_name").unwrap().to_string(),
			p2_name: res.get("p2_name").unwrap().to_string(),
			p3_name: res.get("p3_name").unwrap().to_string(),
			pd1,
			pd2,
			pd3,
			you: res.get("you").unwrap().to_string(),
			game_id: res.get("game_id").unwrap().to_string(),
			room: res.get("room").unwrap().to_string(),
			rules: res.get("rules").unwrap().to_string(),
		})
	}
}
