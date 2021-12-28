use std::convert::{TryFrom, TryInto};
use std::fmt::{Display, Formatter};
use std::fmt;


pub type WindowsScanCode = u16;

/// An event that represent the state change of a key or mouse button.
/// Created by calling `to_key_event()` on a `InputEvent`
#[derive(Copy, Clone, Debug)]
pub struct KeyEvent {
    pub key: VirtualKey,
    pub state: KeyState
}

/// Possible input events
#[derive(Copy, Clone, Debug)]
pub enum InputEvent {
    /// A key got pressed, released or repeated
    KeyboardKeyEvent(VirtualKey, WindowsScanCode, KeyState),
    /// A mouse button got pressed
    MouseButtonEvent(VirtualKey, KeyState),
    /// The mouse wheel moved
    ///
    /// See `ScrollDirection` for more info
    MouseWheelEvent(ScrollDirection),
    /// The mouse moved
    ///
    /// x,y - the new coordinates in pixels
    MouseMoveEvent(i32, i32)
}

impl InputEvent {

    /// Return `Some(KeyEvent)` if the `InputEvent` is either `KeyboardKeyEvent` or `MouseButtonEvent`
    /// and `None` otherwise
    pub fn to_key_event(&self) -> Option<KeyEvent> {
        match self {
            InputEvent::KeyboardKeyEvent(key, _, state) => Some(KeyEvent{
                key: *key,
                state: *state
            }),
            InputEvent::MouseButtonEvent(key, state) => Some(KeyEvent{
                key: *key,
                state: *state
            }),
            _ => None
        }
    }
}

/// This enum describes the kind of input event that should be simulated
///
/// Each Input enum gets translated to one (or in the case of `StringInput` many)
/// [INPUT](https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-input) structs
#[derive(Copy, Clone, Debug)]
pub enum Input<'a> {
    /// Simulates the press or release of a single key
    KeyboardKeyInput(VirtualKey, KeyState),
    /// Send the characters of the given string to the currently active window
    StringInput(&'a str),
    /// Simulates the press or release of a mouse button
    MouseButtonInput(VirtualKey, KeyState),
    /// Simulates the movement of the scroll wheel
    MouseScrollInput(ScrollDirection),
    /// Moves the mouse relative to its current position
    ///
    /// # Arguments
    /// * x, y - The offset in pixels
    RelativeMouseMoveInput(i32, i32),
    /// Moves to mouse to an absolute position
    ///
    /// # Arguments
    /// * x, y - The new mouse coordinates in pixels.
    /// (0,0) is the top-left corner of the primary monitor
    AbsoluteMouseMoveInput(i32, i32)
}

/// The mouse wheel scroll value
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrollDirection {
    /// A horizontal scroll value
    ///
    /// -1.0 means one click to the left, 1.0 one click to the right
    Horizontal(f32),
    /// A vertical scroll value
    ///
    /// -1.0 means one click towards the user, 1.0 one click award from the user
    Vertical(f32)
}

/// The state of a key or mouse button
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum KeyState {
    Pressed, Released
}

/// Windows virtual key code.
///
/// See [Virtual-Key Codes](https://msdn.microsoft.com/en-us/library/windows/desktop/dd375731.aspx) for more information.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum VirtualKey {
    LButton                      = 0x01,
    RButton                      = 0x02,
    Cancel                       = 0x03,
    MButton                      = 0x04,
    XButton1                     = 0x05,
    XButton2                     = 0x06,
    Back                         = 0x08,
    Tab                          = 0x09,
    Clear                        = 0x0C,
    Return                       = 0x0D,
    Shift                        = 0x10,
    Control                      = 0x11,
    Menu                         = 0x12,
    Pause                        = 0x13,
    Capital                      = 0x14,
    KanaHangeulHangul            = 0x15,
    ImeOn                        = 0x16,
    Junja                        = 0x17,
    Final                        = 0x18,
    HanjaKanji                   = 0x19,
    ImeOff                       = 0x1A,
    Escape                       = 0x1B,
    Convert                      = 0x1C,
    NonConvert                   = 0x1D,
    Accept                       = 0x1E,
    ModeChange                   = 0x1F,
    Space                        = 0x20,
    Prior                        = 0x21,
    Next                         = 0x22,
    End                          = 0x23,
    Home                         = 0x24,
    Left                         = 0x25,
    Up                           = 0x26,
    Right                        = 0x27,
    Down                         = 0x28,
    Select                       = 0x29,
    Print                        = 0x2A,
    Execute                      = 0x2B,
    Snapshot                     = 0x2C,
    Insert                       = 0x2D,
    Delete                       = 0x2E,
    Help                         = 0x2F,
    Key0                         = 0x30,
    Key1                         = 0x31,
    Key2                         = 0x32,
    Key3                         = 0x33,
    Key4                         = 0x34,
    Key5                         = 0x35,
    Key6                         = 0x36,
    Key7                         = 0x37,
    Key8                         = 0x38,
    Key9                         = 0x39,
    KeyA                         = 0x41,
    KeyB                         = 0x42,
    KeyC                         = 0x43,
    KeyD                         = 0x44,
    KeyE                         = 0x45,
    KeyF                         = 0x46,
    KeyG                         = 0x47,
    KeyH                         = 0x48,
    KeyI                         = 0x49,
    KeyJ                         = 0x4A,
    KeyK                         = 0x4B,
    KeyL                         = 0x4C,
    KeyM                         = 0x4D,
    KeyN                         = 0x4E,
    KeyO                         = 0x4F,
    KeyP                         = 0x50,
    KeyQ                         = 0x51,
    KeyR                         = 0x52,
    KeyS                         = 0x53,
    KeyT                         = 0x54,
    KeyU                         = 0x55,
    KeyV                         = 0x56,
    KeyW                         = 0x57,
    KeyX                         = 0x58,
    KeyY                         = 0x59,
    KeyZ                         = 0x5A,
    LWin                         = 0x5B,
    RWin                         = 0x5C,
    Apps                         = 0x5D,
    Sleep                        = 0x5F,
    Numpad0                      = 0x60,
    Numpad1                      = 0x61,
    Numpad2                      = 0x62,
    Numpad3                      = 0x63,
    Numpad4                      = 0x64,
    Numpad5                      = 0x65,
    Numpad6                      = 0x66,
    Numpad7                      = 0x67,
    Numpad8                      = 0x68,
    Numpad9                      = 0x69,
    Multiply                     = 0x6A,
    Add                          = 0x6B,
    Separator                    = 0x6C,
    Subtract                     = 0x6D,
    Decimal                      = 0x6E,
    Divide                       = 0x6F,
    F1                           = 0x70,
    F2                           = 0x71,
    F3                           = 0x72,
    F4                           = 0x73,
    F5                           = 0x74,
    F6                           = 0x75,
    F7                           = 0x76,
    F8                           = 0x77,
    F9                           = 0x78,
    F10                          = 0x79,
    F11                          = 0x7A,
    F12                          = 0x7B,
    F13                          = 0x7C,
    F14                          = 0x7D,
    F15                          = 0x7E,
    F16                          = 0x7F,
    F17                          = 0x80,
    F18                          = 0x81,
    F19                          = 0x82,
    F20                          = 0x83,
    F21                          = 0x84,
    F22                          = 0x85,
    F23                          = 0x86,
    F24                          = 0x87,
    NavigationView               = 0x88,
    NavigationMenu               = 0x89,
    NavigationUp                 = 0x8A,
    NavigationDown               = 0x8B,
    NavigationLeft               = 0x8C,
    NavigationRight              = 0x8D,
    NavigationAccept             = 0x8E,
    NavigationCancel             = 0x8F,
    Numlock                      = 0x90,
    Scroll                       = 0x91,
    OemNecEqualFjJisho           = 0x92,
    OemFjMasshou                 = 0x93,
    OemFjTouroku                 = 0x94,
    OemFjLoya                    = 0x95,
    OemFjRoya                    = 0x96,
    LShift                       = 0xA0,
    RShift                       = 0xA1,
    LControl                     = 0xA2,
    RControl                     = 0xA3,
    LMenu                        = 0xA4,
    RMenu                        = 0xA5,
    BrowserBack                  = 0xA6,
    BrowserForward               = 0xA7,
    BrowserRefresh               = 0xA8,
    BrowserStop                  = 0xA9,
    BrowserSearch                = 0xAA,
    BrowserFavorites             = 0xAB,
    BrowserHome                  = 0xAC,
    VolumeMute                   = 0xAD,
    VolumeDown                   = 0xAE,
    VolumeUp                     = 0xAF,
    MediaNextTrack               = 0xB0,
    MediaPrevTrack               = 0xB1,
    MediaStop                    = 0xB2,
    MediaPlayPause               = 0xB3,
    LaunchMail                   = 0xB4,
    LaunchMediaSelect            = 0xB5,
    LaunchApp1                   = 0xB6,
    LaunchApp2                   = 0xB7,
    Oem1                         = 0xBA,
    OemPlus                      = 0xBB,
    OemComma                     = 0xBC,
    OemMinus                     = 0xBD,
    OemPeriod                    = 0xBE,
    Oem2                         = 0xBF,
    Oem3                         = 0xC0,
    GamepadA                     = 0xC3,
    GamepadB                     = 0xC4,
    GamepadX                     = 0xC5,
    GamepadY                     = 0xC6,
    GamepadRightShoulder         = 0xC7,
    GamepadLeftShoulder          = 0xC8,
    GamepadLeftTrigger           = 0xC9,
    GamepadRightTrigger          = 0xCA,
    GamepadDPadUp                = 0xCB,
    GamepadDPadDown              = 0xCC,
    GamepadDPadLeft              = 0xCD,
    GamepadDPadRight             = 0xCE,
    GamepadMenu                  = 0xCF,
    GamepadView                  = 0xD0,
    GamepadLeftThumbStickButton  = 0xD1,
    GamepadRightThumbStickButton = 0xD2,
    GamepadLeftThumbStickUp      = 0xD3,
    GamepadLeftThumbStickDown    = 0xD4,
    GamepadLeftThumbStickRight   = 0xD5,
    GamepadLeftThumbStickLeft    = 0xD6,
    GamepadRightThumbStickUp     = 0xD7,
    GamepadRightThumbStickDown   = 0xD8,
    GamepadRightThumbStickRight  = 0xD9,
    GamepadRightThumbStickLeft   = 0xDA,
    Oem4                         = 0xDB,
    Oem5                         = 0xDC,
    Oem6                         = 0xDD,
    Oem7                         = 0xDE,
    Oem8                         = 0xDF,
    OemAx                        = 0xE1,
    Oem102                       = 0xE2,
    IcoHelp                      = 0xE3,
    Ico00                        = 0xE4,
    ProcessKey                   = 0xE5,
    IcoClear                     = 0xE6,
    Packet                       = 0xE7,
    OemReset                     = 0xE9,
    OemJump                      = 0xEA,
    OemPa1                       = 0xEB,
    OemPa2                       = 0xEC,
    OemPa3                       = 0xED,
    OemWsCtrl                    = 0xEE,
    OemCusel                     = 0xEF,
    OemAttn                      = 0xF0,
    OemFinish                    = 0xF1,
    OemCopy                      = 0xF2,
    OemAuto                      = 0xF3,
    OemEnlw                      = 0xF4,
    OemBackTab                   = 0xF5,
    Attn                         = 0xF6,
    Crsel                        = 0xF7,
    Exsel                        = 0xF8,
    Ereof                        = 0xF9,
    Play                         = 0xFA,
    Zoom                         = 0xFB,
    Noname                       = 0xFC,
    Pa1                          = 0xFD,
    OemClear                     = 0xFE
}

impl VirtualKey {

    pub fn is_mouse_button(self) -> bool {
        matches!(self, VirtualKey::LButton | VirtualKey::MButton | VirtualKey::RButton | VirtualKey::XButton1 | VirtualKey::XButton2)
    }

}

//#[cfg(feature = "serde")]
//impl Serialize for VirtualKey {
//    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where S: Serializer {
//        todo!()
//    }
//}

impl Display for VirtualKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {

        write!(f, "{}", format!("{:?}", self).trim_start_matches("Key"))
    }
}

const INVALID_KEYS: [u8; 60] = [
    0x07, 0x0A, 0x0B, 0x0E, 0x0F, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x5E, 0x88, 0x89,
    0x8A, 0x8B, 0x8C, 0x8D, 0x8E, 0x8F, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F,
    0xB8, 0xB9, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD,
    0xCE, 0xCF, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE0, 0xE8
];

impl TryFrom<u8> for VirtualKey {
    type Error = &'static str;
    fn try_from(id: u8) -> Result<Self, Self::Error> {
        if INVALID_KEYS.contains(&id) {
            Err("Invalid key code!")
        } else {
            unsafe {
                Ok(*(&id as *const u8 as *const VirtualKey))
            }
        }
    }
}

impl TryFrom<u16> for VirtualKey {
    type Error = &'static str;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match TryInto::<u8>::try_into(value).ok() {
            None => Err("Invalid key code!"),
            Some(i) => TryInto::<VirtualKey>::try_into(i)
        }
    }
}

impl TryFrom<u32> for VirtualKey {
    type Error = &'static str;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match TryInto::<u8>::try_into(value).ok() {
            None => Err("Invalid key code!"),
            Some(i) => TryInto::<VirtualKey>::try_into(i)
        }
    }
}

impl From<VirtualKey> for u8 {
    fn from(key: VirtualKey) -> Self {
        unsafe {
            *(&key as *const VirtualKey as *const u8)
        }
    }
}

impl From<VirtualKey> for u16 {
    fn from(key: VirtualKey) -> Self {
        Into::<u8>::into(key).into()
    }
}

impl From<VirtualKey> for u32 {
    fn from(key: VirtualKey) -> Self {
        Into::<u8>::into(key).into()
    }
}