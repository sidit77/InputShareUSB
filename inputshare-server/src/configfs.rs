use std::{env, fs};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::os::unix;

const KEYBOARD_REPORT_DESC: &[u8] = &[
    0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
    0x09, 0x06,        // Usage (Keyboard)
    0xA1, 0x01,        // Collection (Application)
    0x05, 0x07,        //   Usage Page (Kbrd/Keypad)
    0x19, 0xE0,        //   Usage Minimum (0xE0)
    0x29, 0xE7,        //   Usage Maximum (0xE7)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x08,        //   Report Count (8)
    0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x01,        //   Report Count (1)
    0x75, 0x08,        //   Report Size (8)
    0x81, 0x03,        //   Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x05,        //   Report Count (5)
    0x75, 0x01,        //   Report Size (1)
    0x05, 0x08,        //   Usage Page (LEDs)
    0x19, 0x01,        //   Usage Minimum (Num Lock)
    0x29, 0x05,        //   Usage Maximum (Kana)
    0x91, 0x02,        //   Output (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0x95, 0x01,        //   Report Count (1)
    0x75, 0x03,        //   Report Size (3)
    0x91, 0x03,        //   Output (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0x95, 0x06,        //   Report Count (6)
    0x75, 0x08,        //   Report Size (8)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x65,        //   Logical Maximum (101)
    0x05, 0x07,        //   Usage Page (Kbrd/Keypad)
    0x19, 0x00,        //   Usage Minimum (0x00)
    0x29, 0x65,        //   Usage Maximum (0x65)
    0x81, 0x00,        //   Input (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0,              // End Collection
];

const MOUSE_REPORT_DESC: &[u8] = &[
    0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
    0x09, 0x02,        // Usage (Mouse)
    0xA1, 0x01,        // Collection (Application)
    0x09, 0x01,        //   Usage (Pointer)
    0xA1, 0x00,        //   Collection (Physical)
    0x05, 0x09,        //     Usage Page (Button)
    0x19, 0x01,        //     Usage Minimum (0x01)
    0x29, 0x05,        //     Usage Maximum (0x05)
    0x15, 0x00,        //     Logical Minimum (0)
    0x25, 0x01,        //     Logical Maximum (1)
    0x95, 0x05,        //     Report Count (5)
    0x75, 0x01,        //     Report Size (1)
    0x81, 0x02,        //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x01,        //     Report Count (1)
    0x75, 0x03,        //     Report Size (3)
    0x81, 0x03,        //     Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x05, 0x01,        //     Usage Page (Generic Desktop Ctrls)
    0x09, 0x30,        //     Usage (X)
    0x09, 0x31,        //     Usage (Y)
    0x16, 0x00, 0x80,  //     Logical Minimum (-32768)
    0x26, 0xFF, 0x7F,  //     Logical Maximum (32767)
    0x75, 0x10,        //     Report Size (16)
    0x95, 0x02,        //     Report Count (2)
    0x81, 0x06,        //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0x09, 0x38,        //     Usage (Wheel)
    0x15, 0x81,        //     Logical Minimum (-127)
    0x25, 0x7F,        //     Logical Maximum (127)
    0x95, 0x01,        //     Report Count (1)
    0x75, 0x08,        //     Report Size (8)
    0x81, 0x06,        //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0x05, 0x0C,        //     Usage Page (Consumer)
    0x0A, 0x38, 0x02,  //     Usage (AC Pan)
    0x81, 0x06,        //     Input (Data,Var,Rel,No Wrap,Linear,Preferred State,No Null Position)
    0xC0,              //   End Collection
    0xC0,              // End Collection
];

const CONSUMER_REPORT_DESC: &[u8] = &[
    0x05, 0x0C,        // Usage Page (Consumer)
    0x09, 0x01,        // Usage (Consumer Control)
    0xA1, 0x01,        // Collection (Application)
    0x05, 0x0C,        //   Usage Page (Consumer)
    0x15, 0x00,        //   Logical Minimum (0)
    0x25, 0x01,        //   Logical Maximum (1)
    0x75, 0x01,        //   Report Size (1)
    0x95, 0x10,        //   Report Count (16)
    0x09, 0xB5,        //   Usage (Scan Next Track)
    0x09, 0xB6,        //   Usage (Scan Previous Track)
    0x09, 0xB7,        //   Usage (Stop)
    0x09, 0xCD,        //   Usage (Play / Pause)
    0x09, 0xE2,        //   Usage (Mute)
    0x09, 0xE9,        //   Usage (Volume Up)
    0x09, 0xEA,        //   Usage (Volume Down)
    0x0A, 0x23, 0x02,  //   Usage (WWW Home)
    0x0A, 0x94, 0x01,  //   Usage (My Computer)
    0x0A, 0x92, 0x01,  //   Usage (Calculator)
    0x0A, 0x2A, 0x02,  //   Usage (WWW fav)
    0x0A, 0x21, 0x02,  //   Usage (WWW search)
    0x0A, 0x26, 0x02,  //   Usage (WWW stop)
    0x0A, 0x24, 0x02,  //   Usage (WWW back)
    0x0A, 0x83, 0x01,  //   Usage (Media sel)
    0x0A, 0x8A, 0x01,  //   Usage (Mail)
    0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0xC0,              // End Collection
];

pub fn enable_hid() -> Result<()>{
    println!("Enabling HID device");

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
        .filter_map(|r|r
            .map(|e|e.file_name())
            .ok())
        .next()
        .ok_or_else(|| Error::new(ErrorKind::Other, "No UDC found"))?
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "UDC has an invalid name"))?
        .to_string();

    fs::write("UDC", &udc_name)?;

    Ok(())
}

pub fn disable_hid() -> Result<()>{
    println!("Disabling HID device");

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