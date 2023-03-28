use std::num::NonZeroU8;
#[cfg(unix)]
use std::os::unix;
use std::path::Path;
use std::sync::Mutex;
use std::{env, fs};

use anyhow::{anyhow, Result};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;

#[cfg(windows)]
mod unix {
    pub mod fs {
        use std::path::Path;
        pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(_: P, _: Q) -> std::io::Result<()> {
            panic!("not supported on windows");
        }
    }
}

const KEYBOARD_REPORT_DESC: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop Ctrls)
    0x09, 0x06, // Usage (Keyboard)
    0xA1, 0x01, // Collection (Application)
    0x05, 0x07, //   Usage Page (Kbrd/Keypad)
    0x19, 0xE0, //   Usage Minimum (0xE0)
    0x29, 0xE7, //   Usage Maximum (0xE7)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x08, //   Report Count (8)
    0x81, 0x02, //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x01, //   Report Count (1)
    0x75, 0x08, //   Report Size (8)
    0x81, 0x03, //   Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x05, //   Report Count (5)
    0x75, 0x01, //   Report Size (1)
    0x05, 0x08, //   Usage Page (LEDs)
    0x19, 0x01, //   Usage Minimum (Num Lock)
    0x29, 0x05, //   Usage Maximum (Kana)
    0x91, 0x02, //   Output (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0x95, 0x01, //   Report Count (1)
    0x75, 0x03, //   Report Size (3)
    0x91, 0x03, //   Output (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0x95, 0x06, //   Report Count (6)
    0x75, 0x08, //   Report Size (8)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x65, //   Logical Maximum (101)
    0x05, 0x07, //   Usage Page (Kbrd/Keypad)
    0x19, 0x00, //   Usage Minimum (0x00)
    0x29, 0x65, //   Usage Maximum (0x65)
    0x81, 0x00, //   Input (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0  // End Collection
];

const MOUSE_REPORT_DESC: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop Ctrls)
    0x09, 0x02, // Usage (Mouse)
    0xA1, 0x01, // Collection (Application)
    0x09, 0x01, //   Usage (Pointer)
    0xA1, 0x00, //   Collection (Physical)
    0x05, 0x09, //     Usage Page (Button)
    0x19, 0x01, //     Usage Minimum (0x01)
    0x29, 0x05, //     Usage Maximum (0x05)
    0x15, 0x00, //     Logical Minimum (0)
    0x25, 0x01, //     Logical Maximum (1)
    0x95, 0x05, //     Report Count (5)
    0x75, 0x01, //     Report Size (1)
    0x81, 0x02, //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x01, //     Report Count (1)
    0x75, 0x03, //     Report Size (3)
    0x81, 0x03, //     Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x05, 0x01, //     Usage Page (Generic Desktop Ctrls)
    0x09, 0x30, //     Usage (X)
    0x09, 0x31, //     Usage (Y)
    0x16, 0x00, 0x80, //     Logical Minimum (-32768)
    0x26, 0xFF, 0x7F, //     Logical Maximum (32767)
    0x75, 0x10, //     Report Size (16)
    0x95, 0x02, //     Report Count (2)
    0x81, 0x06, //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0x09, 0x38, //     Usage (Wheel)
    0x15, 0x81, //     Logical Minimum (-127)
    0x25, 0x7F, //     Logical Maximum (127)
    0x95, 0x01, //     Report Count (1)
    0x75, 0x08, //     Report Size (8)
    0x81, 0x06, //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0x05, 0x0C, //     Usage Page (Consumer)
    0x0A, 0x38, 0x02, //     Usage (AC Pan)
    0x81, 0x06, //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0xC0, //   End Collection
    0xC0  // End Collection
];

const CONSUMER_REPORT_DESC: &[u8] = &[
    0x05, 0x0C, // Usage Page (Consumer)
    0x09, 0x01, // Usage (Consumer Control)
    0xA1, 0x01, // Collection (Application)
    0x05, 0x0C, //   Usage Page (Consumer)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x10, //   Report Count (16)
    0x09, 0xB5, //   Usage (Scan Next Track)
    0x09, 0xB6, //   Usage (Scan Previous Track)
    0x09, 0xB7, //   Usage (Stop)
    0x09, 0xCD, //   Usage (Play / Pause)
    0x09, 0xE2, //   Usage (Mute)
    0x09, 0xE9, //   Usage (Volume Up)
    0x09, 0xEA, //   Usage (Volume Down)
    0x0A, 0x23, 0x02, //   Usage (WWW Home)
    0x0A, 0x94, 0x01, //   Usage (My Computer)
    0x0A, 0x92, 0x01, //   Usage (Calculator)
    0x0A, 0x2A, 0x02, //   Usage (WWW fav)
    0x0A, 0x21, 0x02, //   Usage (WWW search)
    0x0A, 0x26, 0x02, //   Usage (WWW stop)
    0x0A, 0x24, 0x02, //   Usage (WWW back)
    0x0A, 0x83, 0x01, //   Usage (Media sel)
    0x0A, 0x8A, 0x01, //   Usage (Mail)
    0x81, 0x02, //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0  // End Collection
];

fn enable_hid() -> Result<()> {
    tracing::debug!("Enabling HID device");

    env::set_current_dir("/sys/kernel/config/usb_gadget/")?;
    fs::create_dir("g1")?;
    env::set_current_dir("g1")?;

    fs::write("idVendor", "0x1d6b")?;
    fs::write("idProduct", "0x0104")?;
    fs::write("bcdDevice", "0x0100")?;
    fs::write("bcdUSB", "0x0200")?;

    fs::write("bDeviceClass", "0xef")?;
    fs::write("bDeviceSubClass", "0x02")?;
    fs::write("bDeviceProtocol", "0x01")?;

    fs::create_dir_all("strings/0x409")?;
    fs::write("strings/0x409/serialnumber", "fedcba9876543210")?;
    fs::write("strings/0x409/manufacturer", "sidit77")?;
    fs::write("strings/0x409/product", "InputShareUSB")?;

    fs::create_dir_all("configs/c.1/strings/0x409")?;
    fs::write("configs/c.1/strings/0x409/configuration", "Config 1: Keyboard")?;
    fs::write("configs/c.1/bmAttributes", "0x80")?;
    fs::write("configs/c.1/MaxPower", "250")?;

    fs::create_dir_all("functions/hid.usb0")?;
    fs::write("functions/hid.usb0/protocol", "1")?;
    fs::write("functions/hid.usb0/subclass", "1")?;
    fs::write("functions/hid.usb0/report_length", "8")?;
    fs::write("functions/hid.usb0/report_desc", KEYBOARD_REPORT_DESC)?;
    unix::fs::symlink("functions/hid.usb0", "configs/c.1/hid.usb0")?;

    fs::create_dir_all("functions/hid.usb1")?;
    fs::write("functions/hid.usb1/protocol", "1")?;
    fs::write("functions/hid.usb1/subclass", "1")?;
    fs::write("functions/hid.usb1/report_length", "7")?;
    fs::write("functions/hid.usb1/report_desc", MOUSE_REPORT_DESC)?;
    unix::fs::symlink("functions/hid.usb1", "configs/c.1/hid.usb1")?;

    fs::create_dir_all("functions/hid.usb2")?;
    fs::write("functions/hid.usb2/protocol", "1")?;
    fs::write("functions/hid.usb2/subclass", "1")?;
    fs::write("functions/hid.usb2/report_length", "2")?;
    fs::write("functions/hid.usb2/report_desc", CONSUMER_REPORT_DESC)?;
    unix::fs::symlink("functions/hid.usb2", "configs/c.1/hid.usb2")?;

    fs::write("os_desc/use", "1")?;
    fs::write("os_desc/b_vendor_code", "0xcd")?;
    fs::write("os_desc/qw_sign", "MSFT100")?;
    unix::fs::symlink("configs/c.1", "os_desc/c.1")?;

    let udc_name = Path::new("/sys/class/udc/")
        .read_dir()?
        .filter_map(|r| r.map(|e| e.file_name()).ok())
        .next()
        .ok_or_else(|| anyhow!("No UDC found"))?
        .to_str()
        .ok_or_else(|| anyhow!("UDC has an invalid name"))?
        .to_string();

    fs::write("UDC", udc_name)?;

    Ok(())
}

fn disable_hid() -> Result<()> {
    tracing::debug!("Disabling HID device");

    env::set_current_dir("/sys/kernel/config/usb_gadget/g1/")?;

    fs::write("UDC", "")?;

    fs::remove_file("os_desc/c.1")?;
    fs::remove_file("configs/c.1/hid.usb0")?;
    fs::remove_file("configs/c.1/hid.usb1")?;
    fs::remove_file("configs/c.1/hid.usb2")?;

    fs::remove_dir("configs/c.1/strings/0x409")?;
    fs::remove_dir("configs/c.1")?;
    fs::remove_dir("functions/hid.usb0")?;
    fs::remove_dir("functions/hid.usb1")?;
    fs::remove_dir("functions/hid.usb2")?;
    fs::remove_dir("strings/0x409")?;

    env::set_current_dir("..")?;
    fs::remove_dir("g1")?;

    Ok(())
}

static CONFIG_FS_REF_COUNT: Mutex<u32> = Mutex::new(0);

#[derive(Debug)]
struct ConfigFsHandle;

impl ConfigFsHandle {
    #[allow(unreachable_code)]
    fn new() -> Result<Self> {
        #[cfg(windows)]
        panic!("Not supported on windows");

        let mut guard = CONFIG_FS_REF_COUNT.lock().expect("Could not acquire lock");
        if *guard == 0 {
            enable_hid()?;
        }
        *guard += 1;
        assert_ne!(*guard, 0);
        Ok(Self)
    }
}

impl Drop for ConfigFsHandle {
    fn drop(&mut self) {
        let mut guard = CONFIG_FS_REF_COUNT.lock().expect("Could not acquire lock");
        assert_ne!(*guard, 0);
        *guard -= 1;
        if *guard == 0 {
            if let Err(err) = disable_hid() {
                tracing::error!("Could not remove config fs configuration: {}", err);
            }
        }
    }
}

pub async fn asyncify<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static
{
    match spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(anyhow!("background task failed"))
    }
}

#[derive(Debug)]
pub struct Keyboard {
    _handle: ConfigFsHandle,
    device: File,
    pressed_keys: Vec<HidKeyCode>,
    pressed_modifiers: HidModifierKeys
}

impl Keyboard {
    pub async fn new() -> Result<Self> {
        let _handle = asyncify(ConfigFsHandle::new).await?;
        let device = OpenOptions::new()
            .write(true)
            .append(true)
            .open("/dev/hidg0")
            .await?;
        Ok(Self {
            _handle,
            device,
            pressed_keys: Vec::new(),
            pressed_modifiers: HidModifierKeys::empty()
        })
    }

    async fn send_report(&mut self) -> Result<()> {
        let mut report = [0u8; 8];
        report[0] = self.pressed_modifiers.bits();

        for (i, key) in self.pressed_keys.iter().enumerate().take(6) {
            report[2 + i] = (*key).into()
        }
        tracing::trace!("Wring keyboard report: {:?}", &report);
        self.device.write_all(&report).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.pressed_keys.clear();
        self.pressed_modifiers = HidModifierKeys::empty();
        self.send_report().await
    }

    pub async fn press_key(&mut self, key: HidKeyCode) -> Result<()> {
        match key.try_into() {
            Ok(modifier) => self.pressed_modifiers.insert(modifier),
            Err(_) => self.pressed_keys.push(key)
        }
        self.send_report().await
    }

    pub async fn release_key(&mut self, key: HidKeyCode) -> Result<()> {
        match key.try_into() {
            Ok(modifier) => self.pressed_modifiers.remove(modifier),
            Err(_) => self.pressed_keys.retain(|k| *k != key)
        }
        self.send_report().await
    }
}

#[derive(Debug)]
pub struct ConsumerDevice {
    _handle: ConfigFsHandle,
    device: File,
    pressed_keys: ConsumerDeviceButtons
}

impl ConsumerDevice {
    pub async fn new() -> Result<Self> {
        let _handle = asyncify(ConfigFsHandle::new).await?;
        let device = OpenOptions::new()
            .write(true)
            .append(true)
            .open("/dev/hidg2")
            .await?;
        Ok(Self {
            _handle,
            device,
            pressed_keys: ConsumerDeviceButtons::empty()
        })
    }

    async fn send_report(&mut self) -> Result<()> {
        tracing::trace!("Wring consumer device report: {:?}", &self.pressed_keys.bits().to_le_bytes());
        self.device
            .write_all(&self.pressed_keys.bits().to_le_bytes())
            .await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.pressed_keys = ConsumerDeviceButtons::empty();
        self.send_report().await
    }

    pub async fn press_key(&mut self, key: ConsumerDeviceCode) -> Result<()> {
        match key.try_into() {
            Ok(key) => {
                self.pressed_keys.insert(key);
                self.send_report().await
            }
            Err(()) => Ok(())
        }
    }

    pub async fn release_key(&mut self, key: ConsumerDeviceCode) -> Result<()> {
        match key.try_into() {
            Ok(key) => {
                self.pressed_keys.remove(key);
                self.send_report().await
            }
            Err(()) => Ok(())
        }
    }
}

#[derive(Debug)]
pub struct Mouse {
    _handle: ConfigFsHandle,
    device: File,
    pressed_buttons: HidMouseButtons,
    tess_factor: i16
}

impl Mouse {
    pub async fn new(tess_factor: NonZeroU8) -> Result<Self> {
        let _handle = asyncify(ConfigFsHandle::new).await?;
        let device = OpenOptions::new()
            .write(true)
            .append(true)
            .open("/dev/hidg1")
            .await?;
        Ok(Self {
            _handle,
            device,
            pressed_buttons: HidMouseButtons::empty(),
            tess_factor: i16::from(tess_factor.get())
        })
    }

    async fn send_report(&mut self, dx: i16, dy: i16, dv: i8, dh: i8) -> Result<()> {
        let mut report = [0u8; 7];
        report[0] = self.pressed_buttons.bits();

        report[1..=2].copy_from_slice(&dx.to_le_bytes());
        report[3..=4].copy_from_slice(&dy.to_le_bytes());
        report[5..=5].copy_from_slice(&dv.to_le_bytes());
        report[6..=6].copy_from_slice(&dh.to_le_bytes());

        tracing::trace!("Wring mouse report: {:?}", &report);
        self.device.write_all(&report).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.pressed_buttons = HidMouseButtons::empty();
        self.send_report(0, 0, 0, 0).await
    }

    pub async fn press_button(&mut self, button: HidButtonCode) -> Result<()> {
        match button.try_into() {
            Ok(button) => {
                self.pressed_buttons.insert(button);
                self.send_report(0, 0, 0, 0).await
            }
            Err(_) => Ok(())
        }
    }

    pub async fn release_button(&mut self, button: HidButtonCode) -> Result<()> {
        match button.try_into() {
            Ok(button) => {
                self.pressed_buttons.remove(button);
                self.send_report(0, 0, 0, 0).await
            }
            Err(_) => Ok(())
        }
    }

    pub async fn move_by(&mut self, mut dx: i16, mut dy: i16) -> Result<()> {
        let sx = abs_max(dx / self.tess_factor, dx.signum());
        let sy = abs_max(dy / self.tess_factor, dy.signum());
        while dx != 0 || dy != 0 {
            let tx = abs_min(dx, sx);
            let ty = abs_min(dy, sy);
            self.send_report(tx, ty, 0, 0).await?;
            dx -= tx;
            dy -= ty;
        }
        Ok(())
    }

    pub async fn scroll_vertical(&mut self, amount: i8) -> Result<()> {
        self.send_report(0, 0, amount, 0).await
    }

    pub async fn scroll_horizontal(&mut self, amount: i8) -> Result<()> {
        self.send_report(0, 0, 0, amount).await
    }
}

fn abs_max(a: i16, b: i16) -> i16 {
    if a.abs() >= b.abs() { a } else { b }
}

fn abs_min(a: i16, b: i16) -> i16 {
    if a.abs() <= b.abs() { a } else { b }
}

pub use flags::{ConsumerDeviceButtons, HidModifierKeys, HidMouseButtons};
use inputshare_common::{ConsumerDeviceCode, HidButtonCode, HidKeyCode};

//#[allow(non_upper_case_globals)]
pub mod flags {
    use bitflags::bitflags;
    bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct HidModifierKeys: u8 {
            const LCtrl   = 0x01;
            const LShift  = 0x02;
            const LAlt    = 0x04;
            const LMeta   = 0x08;
            const RCtrl   = 0x10;
            const RShift  = 0x20;
            const RAlt    = 0x40;
            const RMeta   = 0x80;
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct HidMouseButtons: u8 {
            const LButton = 0x01;
            const RButton = 0x02;
            const MButton = 0x04;
            const Button4 = 0x08;
            const Button5 = 0x10;
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct ConsumerDeviceButtons: u16 {
            const NextTrack        = 0x0001;
            const PreviousTrack    = 0x0002;
            const Stop             = 0x0004;
            const PlayPause        = 0x0008;
            const Mute             = 0x0010;
            const VolumeUp         = 0x0020;
            const VolumeDown       = 0x0040;
            const BrowserHome      = 0x0080;
            const MyComputer       = 0x0100;
            const Calculator       = 0x0200;
            const BrowserFavorites = 0x0400;
            const BrowserSearch    = 0x0800;
            const BrowserStop      = 0x1000;
            const BrowserBack      = 0x2000;
            const MediaSelect      = 0x4000;
            const Mail             = 0x8000;
        }
    }
}

impl TryFrom<ConsumerDeviceCode> for ConsumerDeviceButtons {
    type Error = ();

    fn try_from(value: ConsumerDeviceCode) -> std::result::Result<Self, Self::Error> {
        match value {
            ConsumerDeviceCode::NextTrack => Ok(ConsumerDeviceButtons::NextTrack),
            ConsumerDeviceCode::PreviousTrack => Ok(ConsumerDeviceButtons::PreviousTrack),
            ConsumerDeviceCode::Stop => Ok(ConsumerDeviceButtons::Stop),
            ConsumerDeviceCode::PlayPause => Ok(ConsumerDeviceButtons::PlayPause),
            ConsumerDeviceCode::Mute => Ok(ConsumerDeviceButtons::Mute),
            ConsumerDeviceCode::VolumeUp => Ok(ConsumerDeviceButtons::VolumeUp),
            ConsumerDeviceCode::VolumeDown => Ok(ConsumerDeviceButtons::VolumeDown),
            ConsumerDeviceCode::MediaSelect => Ok(ConsumerDeviceButtons::MediaSelect),
            ConsumerDeviceCode::Mail => Ok(ConsumerDeviceButtons::Mail),
            ConsumerDeviceCode::Calculator => Ok(ConsumerDeviceButtons::Calculator),
            ConsumerDeviceCode::MyComputer => Ok(ConsumerDeviceButtons::MyComputer),
            ConsumerDeviceCode::BrowserSearch => Ok(ConsumerDeviceButtons::BrowserSearch),
            ConsumerDeviceCode::BrowserHome => Ok(ConsumerDeviceButtons::BrowserHome),
            ConsumerDeviceCode::BrowserBack => Ok(ConsumerDeviceButtons::BrowserBack),
            ConsumerDeviceCode::BrowserStop => Ok(ConsumerDeviceButtons::BrowserStop),
            ConsumerDeviceCode::BrowserFavorites => Ok(ConsumerDeviceButtons::BrowserFavorites),
            _ => Err(())
        }
    }
}

impl TryFrom<HidButtonCode> for HidMouseButtons {
    type Error = ();

    fn try_from(value: HidButtonCode) -> std::result::Result<Self, Self::Error> {
        match value {
            HidButtonCode::None => Err(()),
            HidButtonCode::LButton => Ok(HidMouseButtons::LButton),
            HidButtonCode::RButton => Ok(HidMouseButtons::RButton),
            HidButtonCode::MButton => Ok(HidMouseButtons::MButton),
            HidButtonCode::Button4 => Ok(HidMouseButtons::Button4),
            HidButtonCode::Button5 => Ok(HidMouseButtons::Button5)
        }
    }
}

impl TryFrom<HidKeyCode> for HidModifierKeys {
    type Error = ();

    fn try_from(value: HidKeyCode) -> std::result::Result<Self, Self::Error> {
        match value {
            HidKeyCode::LeftCtrl => Ok(HidModifierKeys::LCtrl),
            HidKeyCode::LeftShift => Ok(HidModifierKeys::LShift),
            HidKeyCode::LeftAlt => Ok(HidModifierKeys::LAlt),
            HidKeyCode::LeftMeta => Ok(HidModifierKeys::LMeta),
            HidKeyCode::RightCtrl => Ok(HidModifierKeys::RCtrl),
            HidKeyCode::RightShift => Ok(HidModifierKeys::RShift),
            HidKeyCode::RightAlt => Ok(HidModifierKeys::RAlt),
            HidKeyCode::RightMeta => Ok(HidModifierKeys::RMeta),
            _ => Err(())
        }
    }
}
