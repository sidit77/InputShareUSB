use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use druid::im::Vector;
use druid::{Data, Lens};
use once_cell::sync::Lazy;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use yawi::VirtualKey;

use crate::utils::keyset::VirtualKeySet;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnectionCommand {
    ShutdownServer,
    Disconnect
}

#[derive(Debug, Clone, Serialize, Deserialize, Data, Lens)]
pub struct Hotkey {
    pub modifiers: VirtualKeySet,
    pub trigger: VirtualKey
}

impl Hotkey {
    pub fn new<T: IntoIterator<Item = VirtualKey>>(modifiers: T, trigger: VirtualKey) -> Self {
        Self {
            modifiers: VirtualKeySet::from_iter(modifiers),
            trigger
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Data, Lens)]
pub struct Config {
    pub host_address: String,
    pub hotkey: Hotkey,
    pub blacklist: VirtualKeySet,
    pub show_network_info: bool,
    pub network_send_rate: u32,
    pub mouse_speed_factor: f64
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: "localhost:12345".to_string(),
            hotkey: Hotkey::new([VirtualKey::LControl], VirtualKey::Tab),
            blacklist: VirtualKeySet::from_iter([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ]),
            show_network_info: false,
            network_send_rate: 100,
            mouse_speed_factor: 1.0
        }
    }
}

impl Config {
    pub fn path() -> &'static Path {
        static PATH: Lazy<PathBuf> = Lazy::new(|| {
            let dirs = BaseDirs::new().expect("Can not get base dirs");
            let config_dir = dirs.config_dir();
            config_dir.join("InputShare.ron")
        });
        PATH.as_path()
    }

    pub fn load() -> eyre::Result<Self> {
        let path = Self::path();
        let config: Self = match path.exists() {
            true => {
                let file = std::fs::read_to_string(path)?;
                ron::from_str(&file)?
            }
            false => {
                let conf = Self::default();
                conf.save()?;
                conf
            }
        };
        Ok(config)
    }

    pub fn save(&self) -> eyre::Result<()> {
        let pretty = PrettyConfig::new();
        Ok(std::fs::write(Self::path(), to_string_pretty(self, pretty)?)?)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Data)]
pub enum Side {
    Local,
    Remote
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Data)]
pub enum ConnectionState {
    Connected(Side),
    Connecting,
    #[default]
    Disconnected
}

#[derive(Debug, Clone, Eq, PartialEq, Data)]
pub enum PopupType {
    Searching(Vector<SearchResult>),
    Error(String),
    PressKey
}

#[derive(Debug, Clone, Eq, PartialEq, Data)]
pub struct SearchResult {
    pub addrs: SocketAddr
}

#[derive(Default, Debug, Clone, Data, Lens)]
pub struct AppState {
    pub config: Config,
    pub connection_state: ConnectionState,
    pub enable_shutdown: bool,
    pub popup: Option<PopupType>
}
