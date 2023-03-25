use serde::{Serialize, Deserialize};
use druid::{Data, Lens};
use yawi::VirtualKey;
use crate::utils::keyset::VirtualKeySet;

#[derive(Debug, Clone, Serialize, Deserialize, Data, Lens)]
pub struct Hotkey {
    pub modifiers: VirtualKeySet,
    pub trigger: VirtualKey
}

impl Hotkey {
    pub fn new<T: IntoIterator<Item = VirtualKey>>(modifiers: T, trigger: VirtualKey) -> Self {
        Self { modifiers: VirtualKeySet::from_iter(modifiers), trigger}
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Data, Lens)]
pub struct Config {
    pub hotkey: Hotkey,
    pub blacklist: VirtualKeySet,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: Hotkey::new(None, VirtualKey::Apps),
            blacklist: VirtualKeySet::from_iter([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ]),
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

#[derive(Default, Debug, Clone, Data, Lens)]
pub struct AppState {
    pub config: Config,
    pub connection_state: ConnectionState,
    pub popup: bool
}