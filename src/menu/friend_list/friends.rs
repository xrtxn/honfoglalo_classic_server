use crate::emulator::Emulator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendResponse {
    pub error: String,
    pub data: Friends,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendDetails {
    pub id: String,
    pub name: String,
    pub int_avatar: String,
    pub flag: String,
    pub actleague: String,
    pub xplevel: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Friends {
    pub allitems: Vec<FriendDetails>,
}

impl Emulator for FriendResponse {
    fn emulate(_: String) -> FriendResponse {
        FriendResponse {
            error: "0".to_string(),
            data: Friends {
                allitems: vec![FriendDetails {
                    id: "2".to_string(),
                    name: "Lajos".to_string(),
                    int_avatar: "0".to_string(),
                    flag: "0".to_string(),
                    actleague: "1".to_string(),
                    xplevel: "1".to_string(),
                }],
            },
        }
    }
}
