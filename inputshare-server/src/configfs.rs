use std::{env, fs};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::os::unix;

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
    fs::write("functions/hid.usb0/report_desc", &[
        0x05, 0x01, 0x09, 0x06, 0xa1, 0x01, 0x05, 0x07, 0x19, 0xe0,
        0x29, 0xe7, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x08,
        0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x03, 0x95, 0x05,
        0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02,
        0x95, 0x01, 0x75, 0x03, 0x91, 0x03, 0x95, 0x06, 0x75, 0x08,
        0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65,
        0x81, 0x00, 0xc0])?;
    unix::fs::symlink("functions/hid.usb0", "configs/c.1/hid.usb0")?;

    fs::create_dir_all("functions/hid.usb1")?;
    fs::write("functions/hid.usb1/protocol", "1")?;
    fs::write("functions/hid.usb1/subclass", "1")?;
    fs::write("functions/hid.usb1/report_length", "7")?;
    fs::write("functions/hid.usb1/report_desc", &[
        0x05, 0x01, 0x09, 0x02, 0xa1, 0x01, 0x09, 0x01, 0xa1, 0x00,
        0x05, 0x09, 0x19, 0x01, 0x29, 0x05, 0x15, 0x00, 0x25, 0x01,
        0x95, 0x05, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x03,
        0x81, 0x03, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x16, 0x00,
        0x80, 0x26, 0xff, 0x7f, 0x75, 0x10, 0x95, 0x02, 0x81, 0x06,
        0x09, 0x38, 0x15, 0x81, 0x25, 0x7f, 0x95, 0x01, 0x75, 0x08,
        0x81, 0x06, 0x05, 0x0c, 0x0a, 0x38, 0x02, 0x81, 0x06, 0xc0,
        0xc0])?;
    unix::fs::symlink("functions/hid.usb1", "configs/c.1/hid.usb1")?;

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
        .ok_or(Error::new(ErrorKind::Other, "No UDC found"))?
        .to_str()
        .ok_or(Error::new(ErrorKind::InvalidData, "UDC has an invalid name"))?
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

    fs::remove_dir("configs/c.1/strings/0x409")?;
    fs::remove_dir("configs/c.1")?;
    fs::remove_dir("functions/hid.usb0")?;
    fs::remove_dir("functions/hid.usb1")?;
    fs::remove_dir("strings/0x409")?;

    env::set_current_dir("..")?;
    fs::remove_dir("g1")?;

    Ok(())
}