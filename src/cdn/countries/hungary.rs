use crate::cdn::countries::hungary::flags::HunFlags;
use crate::cdn::countries::FlagIds::Hungarian;
use crate::cdn::countries::{CountriesData, CountriesResponse};
use crate::emulator::Emulator;

pub mod flags {
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, Debug)]
	pub enum HunFlags {
		#[serde(rename = "za")]
		Zala,
		#[serde(rename = "ve")]
		Veszprem,
		#[serde(rename = "va")]
		Vas,
		#[serde(rename = "to")]
		Tolna,
		#[serde(rename = "sz")]
		SzabolcsSzatmarBereg,
		#[serde(rename = "so")]
		Somogy,
		#[serde(rename = "pe")]
		Pest,
		#[serde(rename = "no")]
		Nograd,
		#[serde(rename = "ko")]
		KomaromEsztergom,
		#[serde(rename = "ja")]
		JaszNagykunSzolnok,
		#[serde(rename = "he")]
		Heves,
		#[serde(rename = "ha")]
		HajduBihar,
		#[serde(rename = "gy")]
		GyorMosonSopron,
		#[serde(rename = "fe")]
		Fejer,
		#[serde(rename = "cs")]
		CsongradCsanad,
		#[serde(rename = "bo")]
		BorsodAbaujZemplen,
		#[serde(rename = "be")]
		Bekes,
		#[serde(rename = "ba")]
		Baranya,
		#[serde(rename = "bk")]
		BacsKiskun,
		// IDK why these are here
		#[serde(rename = "b1")]
		B1,
		#[serde(rename = "b2")]
		B2,
		#[serde(rename = "b3")]
		B3,
		#[serde(rename = "-o")]
		Unknown1,
		#[serde(rename = "--")]
		Unknown2,
	}
}

impl Emulator for CountriesResponse {
	fn emulate() -> Self {
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
