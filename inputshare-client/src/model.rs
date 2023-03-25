use serde::{Serialize, Deserialize};
use druid::{Data};
use druid::im::HashSet;
use yawi::VirtualKey;

#[derive(Debug, Clone, Serialize, Deserialize, Data)]
pub struct Hotkey {
    pub modifiers: HashSet<VirtualKey>,
    pub trigger: VirtualKey
}

impl Hotkey {
    pub fn new<T: IntoIterator<Item = VirtualKey>>(modifiers: T, trigger: VirtualKey) -> Self {
        Self { modifiers: HashSet::from_iter(modifiers), trigger}
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Data)]
pub struct Config {
    pub hotkey: Hotkey,
    pub blacklist: HashSet<VirtualKey>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: Hotkey::new(None, VirtualKey::Apps),
            blacklist: HashSet::from([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ].as_slice()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Data)]
pub enum Side {
    Local, Remote
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Data)]
pub enum ConnectionState {
    Connected(Side),
    Connecting,
    #[default]
    Disconnected
}

#[derive(Default, Debug, Clone, Data)]
pub struct AppState {
    pub config: Config,
    pub connection_state: ConnectionState,
    pub popup: bool
}