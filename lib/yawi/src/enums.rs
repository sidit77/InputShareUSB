use std::char::REPLACEMENT_CHARACTER;
use std::fmt::{Display, Formatter, Write};
use std::fmt;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyNameTextW, MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VIRTUAL_KEY};


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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, TryFromPrimitive, IntoPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "druid", derive(druid::Data))]
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

impl From<VirtualKey> for VIRTUAL_KEY {
    fn from(value: VirtualKey) -> Self {
        VIRTUAL_KEY(u8::from(value).into())
    }
}

impl VirtualKey {
    pub fn is_mouse_button(self) -> bool {
        matches!(self, VirtualKey::LButton | VirtualKey::MButton | VirtualKey::RButton | VirtualKey::XButton1 | VirtualKey::XButton2)
    }
}

impl Display for VirtualKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match KEY_NAMES[u8::from(*self) as usize] {
            None => f.write_str("<UNKNOWN>"),
            Some(name) => match name.contains("OEM") {
                true => {
                    let mut buffer = [0u16; 512];
                    let length = unsafe {
                        let scan_code = MapVirtualKeyW(u8::from(*self) as u32, MAPVK_VK_TO_VSC_EX);
                        let extended = scan_code & 0xFF00 == 0xE100 || scan_code & 0xFF00 == 0xE000;
                        let scan_code = (scan_code & 0xFF) << 16 | u32::from(extended) << 24;
                        GetKeyNameTextW(scan_code as i32, &mut buffer) as usize
                    };
                    let iter = char::decode_utf16(buffer[..length].iter().copied())
                        .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER));
                    let mut start = true;
                    for mut c in iter {
                        if c.is_whitespace() {
                            start = true;
                        } else if start {
                            c = c.to_ascii_uppercase();
                            start = false;
                        } else {
                            c = c.to_ascii_lowercase();
                        }
                        f.write_char(c)?
                    }
                    Ok(())
                }
                false => f.write_str(name)
            }
        }
    }
}

const KEY_NAMES: [Option<&'static str>; 256] = [
    None,
    Some("Left Button"),
    Some("Right Button"),
    Some("Break"),
    Some("Middle Button"),
    Some("X Button 1"),
    Some("X Button 2"),
    None,
    Some("Backspace"),
    Some("Tab"),
    None,
    None,
    Some("Clear"),
    Some("Enter"),
    None,
    None,
    None,
    None,
    None,
    Some("Pause"),
    Some("Caps Lock"),
    Some("Kana"),
    None,
    Some("Junja"),
    Some("Final"),
    Some("Kanji"),
    None,
    Some("Esc"),
    Some("Convert"),
    Some("Non Convert"),
    Some("Accept"),
    Some("Mode Change"),
    Some("Space"),
    Some("Page Up"),
    Some("Page Down"),
    Some("End"),
    Some("Home"),
    Some("Arrow Left"),
    Some("Arrow Up"),
    Some("Arrow Right"),
    Some("Arrow Down"),
    Some("Select"),
    Some("Print"),
    Some("Execute"),
    Some("Print Screen"),
    Some("Insert"),
    Some("Delete"),
    Some("Help"),
    Some("0"),
    Some("1"),
    Some("2"),
    Some("3"),
    Some("4"),
    Some("5"),
    Some("6"),
    Some("7"),
    Some("8"),
    Some("9"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("A"),
    Some("B"),
    Some("C"),
    Some("D"),
    Some("E"),
    Some("F"),
    Some("G"),
    Some("H"),
    Some("I"),
    Some("J"),
    Some("K"),
    Some("L"),
    Some("M"),
    Some("N"),
    Some("O"),
    Some("P"),
    Some("Q"),
    Some("R"),
    Some("S"),
    Some("T"),
    Some("U"),
    Some("V"),
    Some("W"),
    Some("X"),
    Some("Y"),
    Some("Z"),
    Some("Left Win"),
    Some("Right Win"),
    Some("Context Menu"),
    None,
    Some("Sleep"),
    Some("Numpad 0"),
    Some("Numpad 1"),
    Some("Numpad 2"),
    Some("Numpad 3"),
    Some("Numpad 4"),
    Some("Numpad 5"),
    Some("Numpad 6"),
    Some("Numpad 7"),
    Some("Numpad 8"),
    Some("Numpad 9"),
    Some("Numpad *"),
    Some("Numpad +"),
    Some("Separator"),
    Some("Num -"),
    Some("Numpad ."),
    Some("Numpad /"),
    Some("F1"),
    Some("F2"),
    Some("F3"),
    Some("F4"),
    Some("F5"),
    Some("F6"),
    Some("F7"),
    Some("F8"),
    Some("F9"),
    Some("F10"),
    Some("F11"),
    Some("F12"),
    Some("F13"),
    Some("F14"),
    Some("F15"),
    Some("F16"),
    Some("F17"),
    Some("F18"),
    Some("F19"),
    Some("F20"),
    Some("F21"),
    Some("F22"),
    Some("F23"),
    Some("F24"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("Num Lock"),
    Some("Scrol Lock"),
    Some("Jisho"),
    Some("Mashu"),
    Some("Touroku"),
    Some("Loya"),
    Some("Roya"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("Left Shift"),
    Some("Right Shift"),
    Some("Left Ctrl"),
    Some("Right Ctrl"),
    Some("Left Alt"),
    Some("Right Alt"),
    Some("Browser Back"),
    Some("Browser Forward"),
    Some("Browser Refresh"),
    Some("Browser Stop"),
    Some("Browser Search"),
    Some("Browser Favorites"),
    Some("Browser Home"),
    Some("Volume Mute"),
    Some("Volume Down"),
    Some("Volume Up"),
    Some("Next Track"),
    Some("Previous Track"),
    Some("Stop"),
    Some("Play / Pause"),
    Some("Mail"),
    Some("Media"),
    Some("App1"),
    Some("App2"),
    None,
    None,
    Some("OEM_1 (: ;)"),
    Some("OEM_PLUS (+ =)"),
    Some("OEM_COMMA (< ,)"),
    Some("OEM_MINUS (_ -)"),
    Some("OEM_PERIOD (> .)"),
    Some("OEM_2 (? /)"),
    Some("OEM_3 (~ `)"),
    Some("Abnt C1"),
    Some("Abnt C2"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("OEM_4 ({ [)"),
    Some("OEM_5 (| \\)"),
    Some("OEM_6 (} ])"),
    Some("OEM_7 (\" ')"),
    Some("OEM_8 (§ !)"),
    None,
    Some("Ax"),
    Some("OEM_102 (> <)"),
    Some("IcoHlp"),
    Some("Ico00"),
    Some("Process"),
    Some("IcoClr"),
    Some("Packet"),
    None,
    Some("Reset"),
    Some("Jump"),
    Some("OemPa1"),
    Some("OemPa2"),
    Some("OemPa3"),
    Some("WsCtrl"),
    Some("Cu Sel"),
    Some("Oem Attn"),
    Some("Finish"),
    Some("Copy"),
    Some("Auto"),
    Some("Enlw"),
    Some("Back Tab"),
    Some("Attn"),
    Some("Cr Sel"),
    Some("Ex Sel"),
    Some("Er Eof"),
    Some("Play"),
    Some("Zoom"),
    Some("NoName"),
    Some("Pa1"),
    Some("OemClr"),
    Some("no VK mapping")
];
