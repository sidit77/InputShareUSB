use serde::{Serialize, Deserialize};
use yawi::VirtualKey;
use confy::ConfyError;
use inputshare_common::DEFAULT_PORT;

const CONFIG_PATH: &str = concat!(env!("CARGO_CRATE_NAME"), ".toml");

fn print_help() {
    println!("\
USAGE:
  {} [OPTIONS]
FLAGS:
  -h, --help            Prints help information
OPTIONS:
  --address ADDRESS     Overrides the address set in the config",
             env!("CARGO_BIN_NAME"))
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub host_address: String,
    pub hotkey: VirtualKey,
    pub backlist: Vec<VirtualKey>,
    #[serde(skip_serializing)]
    address_override: Option<String>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: format!("raspberrypi.local:{}", DEFAULT_PORT),
            hotkey: VirtualKey::Apps,
            backlist: vec!(
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ),
            address_override: None
        }
    }
}

impl Config {
    pub fn load() ->  anyhow::Result<Self> {
        let mut config: Config = confy::load_path(CONFIG_PATH)?;

        let mut pargs = pico_args::Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print_help();
            std::process::exit(0);
        }

        config.address_override = pargs.opt_value_from_str("--address")?;

        let remaining = pargs.finish();
        if !remaining.is_empty() {
            eprintln!("Warning: unused arguments left: {:?}.", remaining);
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfyError> {
        confy::store_path(CONFIG_PATH, self)
    }

    pub fn merged_address(&self) -> &str {
        match self.address_override {
            None => self.host_address.as_str(),
            Some(ref address) => address.as_str()
        }
    }

}