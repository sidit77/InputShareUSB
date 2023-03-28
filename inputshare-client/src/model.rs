use std::net::SocketAddr;

use druid::im::Vector;
use druid::{Data, Lens};
use serde::{Deserialize, Serialize};
use yawi::VirtualKey;

use crate::utils::keyset::VirtualKeySet;

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
    pub mouse_speed_factor: f32
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
    pub popup: Option<PopupType>
}
