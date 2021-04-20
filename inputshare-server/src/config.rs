use serde::{Serialize, Deserialize};

const CONFIG_PATH: &str = concat!(env!("CARGO_CRATE_NAME"), ".toml");

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub local_address: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
             local_address: String::from("0.0.0.0:12351")
        }
    }
}

impl Config {
    pub fn load() -> Self {
        confy::load_path(CONFIG_PATH).expect("can not load config!")
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        confy::store_path(CONFIG_PATH, self).expect("can not save config!");
    }
}