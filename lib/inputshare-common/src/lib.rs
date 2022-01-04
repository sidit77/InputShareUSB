use std::fmt::Debug;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

pub const IDENTIFIER: &str = concat!(env!("CARGO_CRATE_NAME"), "_" ,env!("CARGO_PKG_VERSION"));

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2<T> where T: Debug + Copy + PartialEq {
    pub x: T,
    pub y: T
}

impl<T> Vec2<T> where T: Debug + Copy + PartialEq {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

pub type MouseType = i64;

#[derive(Debug, Copy, Clone, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MessageType {
    KeyPress,
    KeyRelease,
    MouseButtonPress,
    MouseButtonRelease,
    HorizontalScrolling,
    VerticalScrolling,
    Reset
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum HidButtonCode {
    #[num_enum(default)]
    None = 0x00,
    LButton = 0x01,
    RButton = 0x02,
    MButton = 0x03,
    Button4 = 0x04,
    Button5 = 0x05
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum HidKeyCode {
    #[num_enum(default)]
    None = 0x00,
    ErrorRollOver = 0x01,
    PostFail = 0x02,
    ErrorUndefined = 0x03,

    KeyA = 0x04,
    KeyB = 0x05,
    KeyC = 0x06,
    KeyD = 0x07,
    KeyE = 0x08,
    KeyF = 0x09,
    KeyG = 0x0a,
    KeyH = 0x0b,
    KeyI = 0x0c,
    KeyJ = 0x0d,
    KeyK = 0x0e,
    KeyL = 0x0f,
    KeyM = 0x10,
    KeyN = 0x11,
    KeyO = 0x12,
    KeyP = 0x13,
    KeyQ = 0x14,
    KeyR = 0x15,
    KeyS = 0x16,
    KeyT = 0x17,
    KeyU = 0x18,
    KeyV = 0x19,
    KeyW = 0x1a,
    KeyX = 0x1b,
    KeyY = 0x1c,
    KeyZ = 0x1d,

    Key1 = 0x1e,
    Key2 = 0x1f,
    Key3 = 0x20,
    Key4 = 0x21,
    Key5 = 0x22,
    Key6 = 0x23,
    Key7 = 0x24,
    Key8 = 0x25,
    Key9 = 0x26,
    Key0 = 0x27,

    Enter = 0x28,
    Escape = 0x29,
    Backspace = 0x2a,
    Tab = 0x2b,
    Space = 0x2c,
    Minus = 0x2d,
    Equal = 0x2e,
    LeftBrace = 0x2f,
    RightBrace = 0x30,
    Backslash = 0x31,
    HashTilde = 0x32,
    Semicolon = 0x33,
    Apostrophe = 0x34,
    Grave = 0x35,
    Comma = 0x36,
    Dot = 0x37,
    Slash = 0x38,
    Capslock = 0x39,

    F1 	= 0x3a,
    F2 	= 0x3b,
    F3 	= 0x3c,
    F4 	= 0x3d,
    F5 	= 0x3e,
    F6 	= 0x3f,
    F7 	= 0x40,
    F8 	= 0x41,
    F9 	= 0x42,
    F10 = 0x43,
    F11 = 0x44,
    F12 = 0x45,

    PrintScreen = 0x46,
    ScrollLock = 0x47,
    Pause = 0x48,
    Insert = 0x49,
    Home = 0x4a,
    PageUp = 0x4b,
    Delete = 0x4c,
    End = 0x4d,
    PageDown = 0x4e,
    Right = 0x4f,
    Left = 0x50,
    Down = 0x51,
    Up = 0x52,

    NumLock = 0x53,
    KpSlash = 0x54,
    KpAsterisk = 0x55,
    KpMinus = 0x56,
    KpPlus = 0x57,
    KpEnter = 0x58,
    Kp1 = 0x59,
    Kp2 = 0x5a,
    Kp3 = 0x5b,
    Kp4 = 0x5c,
    Kp5 = 0x5d,
    Kp6 = 0x5e,
    Kp7 = 0x5f,
    Kp8 = 0x60,
    Kp9 = 0x61,
    Kp0 = 0x62,
    KpDot = 0x63,

    Key102ND = 0x64,
    Compose = 0x65,
    Power = 0x66,
    KpEqual = 0x67,

    F13 = 0x68,
    F14 = 0x69,
    F15 = 0x6a,
    F16 = 0x6b,
    F17 = 0x6c,
    F18 = 0x6d,
    F19 = 0x6e,
    F20 = 0x6f,
    F21 = 0x70,
    F22 = 0x71,
    F23 = 0x72,
    F24 = 0x73,

    Execute = 0x74,
    Help = 0x75,
    Menu = 0x76,
    Select = 0x77,
    Stop = 0x78,
    Again = 0x79,
    Undo = 0x7a,
    Cut = 0x7b,
    Copy = 0x7c,
    Paste = 0x7d,
    Find = 0x7e,
    Mute = 0x7f,
    VolumeUp = 0x80,
    VolumeDown = 0x81,

    LockingCapsLock = 0x82,
    LockingNumLock = 0x83,
    LockingScrollLock = 0x84,

    KpComma = 0x85,
    KpEqualSign = 0x86,
    International1 = 0x87,
    International2 = 0x88,
    International3 = 0x89,
    International4 = 0x8a,
    International5 = 0x8b,
    International6 = 0x8c,
    International7 = 0x8d,
    International8 = 0x8e,
    International9 = 0x8f,
    Language1  = 0x90,
    Language2 = 0x91,
    Language3 = 0x92,
    Language4 = 0x93,
    Language5 = 0x94,
    Language6 = 0x95,
    Language7 = 0x96,
    Language8 = 0x97,
    Language9 = 0x98,

    AltErase = 0x99,
    SysReq = 0x9a,
    Cancel = 0x9b,
    Clear = 0x9c,
    Prior = 0x9d,
    Return = 0x9e,
    Separator = 0x9f,
    Out = 0xa0,
    Oper = 0xa1,
    ClearAgain = 0xa2,
    CrSel = 0xa3,
    ExSel = 0xa4,

    // According to QMK, 0xA5-0xDF are not usable on modern keyboards

    LeftCtrl = 0xe0,
    LeftShift = 0xe1,
    LeftAlt = 0xe2,
    LeftMeta = 0xe3,
    RightCtrl = 0xe4,
    RightShift = 0xe5,
    RightAlt = 0xe6,
    RightMeta = 0xe7,

    // Unofficial
    MediaPlayPause = 0xe8,
    MediaStopCD = 0xe9,
    MediaPreviousSong = 0xea,
    MediaNextSong = 0xeb,
    MediaEjectCD = 0xec,
    MediaVolumeUp = 0xed,
    MediaVolumeDown = 0xee,
    MediaMute = 0xef,
    MediaWWW = 0xf0,
    MediaBack = 0xf1,
    MediaForward = 0xf2,
    MediaStop = 0xf3,
    MediaFind = 0xf4,
    MediaScrollUp = 0xf5,
    MediaScrollDown = 0xf6,
    MediaEdit = 0xf7,
    MediaSleep = 0xf8,
    MediaCoffee = 0xf9,
    MediaRefresh = 0xfa,
    MediaCalc = 0xfb,
}
