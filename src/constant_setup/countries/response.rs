use crate::constant_setup::countries::hungary::flags::HunFlags;
use crate::constant_setup::countries::response::FlagIds::Hungarian;
use crate::emulator::Emulator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CountriesResponse {
    pub error: String,
    pub data: Vec<CountriesData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CountriesData {
    pub id: FlagIds,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FlagIds {
    Hungarian(HunFlags),
}

impl Emulator for CountriesResponse {
    fn emulate(_: String) -> Self {
        CountriesResponse {
            error: "0".to_string(),
            data: vec![CountriesData {
                id: Hungarian(HunFlags::SzabolcsSzatmarBereg),
                name: "szabócs".to_string(),
                description: "szabócs megye a vakok világa".to_string(),
            }],
        }
    }
}
