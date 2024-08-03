use crate::emulator::Emulator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename = "ROOT")]
pub struct GameMenuWaithall {
    #[serde(rename = "L")]
    pub l: L,
    #[serde(rename = "STATE")]
    pub state: State,
    #[serde(rename = "GAMEROOM")]
    pub gameroom: Vec<Gameroom>,
    #[serde(rename = "WAITSTATE")]
    pub waitstate: Waitstate,
}

#[derive(Serialize, Deserialize)]
pub struct L {
    #[serde(rename = "@CID")]
    pub cid: String,
    #[serde(rename = "@MN")]
    pub mn: String,
    #[serde(rename = "@R")]
    pub r: String,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "@SCR")]
    pub screen: String,
}

#[derive(Serialize, Deserialize)]
pub struct Gameroom {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@TITLE")]
    pub title: String,
    #[serde(rename = "@MAP")]
    pub map: String,
    #[serde(rename = "@TYPE")]
    pub gameroom_type: String,
    #[serde(rename = "@PLAYERS")]
    pub players: String,
    #[serde(rename = "@INGAME")]
    pub ingame: String,
}

#[derive(Serialize, Deserialize)]
pub struct Waitstate {
    #[serde(rename = "@ROOMSEL")]
    pub roomsel: String,
}

impl Emulator for GameMenuWaithall {
    fn emulate(mn: String) -> Self {
        GameMenuWaithall {
            l: L {
                cid: "1".to_string(),
                mn,
                r: "0".to_string(),
            },
            state: State {
                screen: "WAIT".to_string(),
            },
            gameroom: vec![
                Gameroom {
                    id: "1".to_string(),
                    title: "DEFAULT".to_string(),
                    map: "WD".to_string(),
                    gameroom_type: "2".to_string(),
                    players: "0".to_string(),
                    ingame: "0".to_string(),
                },
                Gameroom {
                    id: "2".to_string(),
                    title: "LONG".to_string(),
                    map: "WD".to_string(),
                    gameroom_type: "2".to_string(),
                    players: "0".to_string(),
                    ingame: "0".to_string(),
                },
            ],
            waitstate: Waitstate {
                roomsel: "0".to_string(),
            },
        }
    }
}
