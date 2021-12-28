# Input Share USB

## Road to 2.0
* Client
- [ ] adaptive sleep duration
- [ ] limit update rate
- [ ] show ping / packet loss
- [ ] client error management
- [ ] new icon?
* Yawi
- [ ] use `std::io::Error`
- [ ] use `threadlocal` for `InputHook`
- [ ] use `num_enum` for `VirtualKey`
* General
- [ ] CI Pipeline
- [ ] Update readme
- [ ] Cleanup old install scripts


## Overview

This program can share your mouse and keyboard with a second device. 

What makes this different from something like [Synergy](https://github.com/symless/synergy-core) is that it uses a Raspberry Pi to act as keyboard and mouse for the second device. So from the perspective from the second device this solution looks like any other USB keyboard and mouse. This allows this program to work in environments where software only solutions can't run such as devices without access to the local network (i.e it uses a VPN) or devices on which you can't or don't want to run external software.

In other words it is designed to run with you typical work laptop.

## Required Hardware

This program needs an external piece of hardware that has some form of network access and can act like a USB-device (supports USB-OTG). I use a [Raspberry Pi Zero W](https://www.raspberrypi.org/products/raspberry-pi-zero-w/), but the [Raspberry Pi 4](https://www.raspberrypi.org/products/raspberry-pi-4-model-b/) should work as well.

## Project Structure

* `InputShareUSB/inputshare-client` : A command line client; Currently the default way of connecting to the server
* `InputShareUSB/inputshare-common`: A library that contains shared code.
* `InputShareUSB/inputshare-gui`: An experimental GUI client based on [Iced](https://crates.io/crates/iced); Currently not functional
* `InputShareUSB/inputshare-server` : The server of this project
* `InputShareUSB/yawi`: **Y**et **A**nother **W**indows **I**nput crate. Handles windows related input stuff for `inputshare-client` and `inputshare-gui`



## Installation

### Raspberry Pi

**Outdated** 

Connect your Raspberry Pi to your second device used the data USB port

Clone repository:

```bash
git clone https://github.com/sidit77/InputShareUSB.git
cd InputShareUSB
```

Build the server:

````bash
cd inputshare-server
cargo build --release
````

Run setup scripts:

````bash
cd scripts

# configure linux hid gadget driver
sudo ./enable_hid.sh

# install server
sudo ./copy_server.sh

# install auto start service for server
sudo ./install_server.sh
````

Reboot and everything should be working:

```bash
sudo reboot
```



### Windows

Clone repository:

```powershell
git clone https://github.com/sidit77/InputShareUSB.git
cd InputShareUSB
```

*Optional:* Edit `inputshare_client.toml` to set the correct server address (default:  `raspberrypi.local:12351`)

Run the client:

````powershell
cargo run --bin inputshare-client --release
````



## Usage

Once everything is running and the client connected to the server successfully simply press the `Apps` key on your keyboard to redirect all mouse and keyboard events to the second device.

![the apps key](https://conemu.github.io/img/KeyboardAppsKey.png)

