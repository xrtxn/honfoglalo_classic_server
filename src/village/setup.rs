use crate::emulator::Emulator;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename = "ROOT")]
pub struct VillageSetupRoot {
    #[serde(rename = "L")]
    pub l: L,
    #[serde(rename = "STATE")]
    pub state: State,
    #[serde(rename = "GAMEPARAMS")]
    pub gameparams: Vec<GameParams>,
    #[serde(rename = "MYDATA")]
    pub mydata: Mydata,
    #[serde(rename = "QCATS")]
    pub question_categories: QuestionCategories,
    #[serde(rename = "FEATURES")]
    pub features: Features,
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
    pub scr: String,
}

#[derive(Serialize, Deserialize)]
pub struct Mydata {
    #[serde(rename = "@NAME")]
    pub name: String,
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@COUNTRY")]
    pub country: String,
    #[serde(rename = "@XPPACK")]
    pub xppack: String,
    #[serde(rename = "@SEX")]
    pub sex: String,
    #[serde(rename = "@GAMECOUNT")]
    pub gamecount: String,
    #[serde(rename = "@GAMECOUNTSR")]
    pub gamecountsr: String,
    #[serde(rename = "@GOLDS")]
    pub golds: String,
    #[serde(rename = "@CASTLELEVEL")]
    pub castlelevel: String,
    #[serde(rename = "@SNDVOL")]
    // sound volume [0-4095]
    pub sounds_volume: String,
    #[serde(rename = "@FLAGS")]
    pub flags: String,
    #[serde(rename = "@MTCUPS")]
    pub mtcups: String,
    #[serde(rename = "@CWINS")]
    pub cwins: String,
    #[serde(rename = "@ENERGYPACK")]
    pub energypack: String,
    #[serde(rename = "@LEVELFLAGS")]
    pub levelflags: String,
    #[serde(rename = "@MISSIONS")]
    pub missions: String,
    #[serde(rename = "@FH")]
    pub fh: String,
    #[serde(rename = "@HP")]
    pub hp: String,
    #[serde(rename = "@SOLDIER")]
    pub soldier: String,
    #[serde(rename = "@SMSR")]
    pub smsr: String,
    #[serde(rename = "@CUSTOMAVATAR")]
    pub customavatar: String,
    #[serde(rename = "@USECUSTOMAVATAR")]
    pub usecustomavatar: String,
    #[serde(rename = "@EXTAVATAR")]
    pub extavatar: String,
    #[serde(rename = "@MYCATEGORY")]
    pub mycategory: String,
    #[serde(rename = "@HFS")]
    pub hfs: String,
    #[serde(rename = "@TAXDATA")]
    pub taxdata: String,
    #[serde(rename = "@LASTPLACES")]
    pub lastplaces: String,
}

//todo
#[derive(Serialize, Deserialize)]
struct HelpForge {
    pub prodtime: i64,
    pub remainingtime: i64,
    pub prodcount: i64,
}

#[derive(Serialize, Deserialize)]
pub struct QuestionCategories {
    #[serde(rename = "@CATEGORIES")]
    pub categories: String,
}

#[derive(Serialize, Deserialize)]
pub struct Features {
    #[serde(rename = "@ENABLED")]
    pub enabled: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameParams {
    #[serde(rename = "@BADGEBONUSES")]
    pub badgebonuses: String,
    #[serde(rename = "@NRG")]
    pub nrg: String,
    #[serde(rename = "@HFUG")]
    pub hfug: String,
    #[serde(rename = "@MP")]
    pub mp: String,
    #[serde(rename = "@HP")]
    pub hp: String,
}

impl Emulator for VillageSetupRoot {
    fn emulate(mn: String) -> Self {
        VillageSetupRoot {
            l: L {
                cid: "1".to_string(),
                mn,
                r: "0".to_string(),
            },
            state: State {
                scr: "VILLAGE".to_string(),
            },
            mydata: Mydata {
                name: "xrtxn".to_string(),
                id: "1".to_string(),
                country: "us".to_string(),
                xppack: "14000, 15, 14000, 18500".to_string(),
                sex: "0".to_string(),
                gamecount: "0".to_string(),
                gamecountsr: "0".to_string(),
                golds: "3000".to_string(),
                castlelevel: "1".to_string(),
                sounds_volume: "3000".to_string(),
                flags: "32768".to_string(),
                mtcups: "1,22,33".to_string(),
                cwins: "1,2,0,1,2,3,0".to_string(),
                energypack: "100,75,0,300,1,0".to_string(),
                levelflags: "0".to_string(),
                missions: "255,255,0".to_string(),
                fh: "5,3,7,6,0,0,0,0,0,0,0,0".to_string(),
                hp: "1000,1000,900,1000,800,1000,900,1000,2000,5000,10000,20000".to_string(),
                soldier: "1".to_string(),
                smsr: "0,0".to_string(),
                customavatar: "".to_string(),
                usecustomavatar: "0".to_string(),
                extavatar: "".to_string(),
                mycategory: "0".to_string(),
                hfs: "1,1,24,32500|0,1,168,86400|0,1,168,30|0,1,168,120000|0,1,168,320000|0,1,168,40000|0,2,168,5|0,1,168,30|0,1,168,30".to_string(),
                taxdata: "4500,10,3000,600,500".to_string(),
                lastplaces: "3211230000".to_string(),
            },
            question_categories: QuestionCategories {
                categories: "1^Art|2^Everydays|3^Geography|4^History|5^Literature|6^Science: Mat-Phy.|7^Science: Bio-Chem|8^Sport|9^Entertainment|10^Lifestyle".to_string(),
            },
            features: Features {
                enabled: "".to_string(),
            },
            gameparams: vec![
                GameParams {
                    badgebonuses: "3,4,5,6,7,8,9|2,3,4,5,6,7,8|1,2,3,4,5,6,7|1,2,3,4,5,6,7|1,2,3,4,5,6,7|1,2,3,4,5,6,7|4,5,6,7,8,9,10|1,0,0,0,0,0,0".to_string(),
                    nrg: "15,3".to_string(),
                    hfug: "1,1,24,0|1,1,24,20000|1,1,24,20000|1,1,24,20000|1,1,24,20000|1,1,24,20000|1,1,24,20000".to_string(),
                    mp: "1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,1000,10,9000,100,70000|1,20000,2,40000,3,60000|1,50000,2,100000,3,150000|1,100000,2,200000,3,300000|1,200000,2,400000,3,600000|1,500000,2,1000000,3,1500000".to_string(),
                    hp: "2000,2000,2000,2000,2000,2000,2000,20000,50000,100000,200000,500000".to_string(),
                }
            ],
        }
    }
}
