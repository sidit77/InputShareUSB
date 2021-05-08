use serde::{Serialize, Deserialize};

const CONFIG_PATH: &str = concat!(env!("CARGO_BIN_NAME"), ".toml");

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("raspberrypi.local"),
            port: 12351
        }
    }
}

impl Config {
    pub fn load() -> Self {
        confy::load_path(CONFIG_PATH).expect("can not load config!")
    }

    pub fn save(&self) {
        confy::store_path(CONFIG_PATH, self).expect("can not store config!");
    }
}

