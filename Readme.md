# Input Share USB



## Overview

This program can share your mouse and keyboard with a second device. 

What makes this different from something like [Synergy](https://github.com/symless/synergy-core) is that it uses a Raspberry Pi to act as keyboard and mouse for the second device. So from the perspective from the second device this solution looks like any other USB keyboard and mouse. This allows this program to work in environments where software only solutions can't run such as devices without access to the local network (i.e it uses a VPN) or devices on which you can't or don't want to run external software.

In other words it is designed to run with you typical work laptop.



## Required Hardware

This program needs an external piece of hardware that has some form of network access and can act like a USB-device (supports USB-OTG). I use a [Raspberry Pi Zero W](https://www.raspberrypi.org/products/raspberry-pi-zero-w/), but the [Raspberry Pi 4](https://www.raspberrypi.org/products/raspberry-pi-4-model-b/) should work as well.
It's also important that the used USB cable is an actual data cable and not just a charging cable (Yes, they unfortunately exist and can be hard to tell apart).



## Usage

### Server

The server run on the raspberry pi and outputs the input commands it receives from the connected client over usb.

### Client

The client runs on windows and allows you to connect to a running server. Once connected you can press the configured hotkey (default: `Apps`) to capture all mouse and keyboard and transmit it to the server. The `Shutdown Server` will attempt to phyiscally shutdown the device that is running the server allowing one to safely unplug the pi.

![MLKf8Wa](https://user-images.githubusercontent.com/5053369/171002399-1ac4d498-21ea-480c-92f7-106b653721a1.png)



## Installation

### Client

#### Step 1 (Option 1; Recommended): Download Prebuild Binary

You can download prebuild binaries from the [Github Release Page](https://github.com/sidit77/InputShareUSB/releases).

Simply unzip the .exe the desired directory and run it. No installation needed.



#### Step 1 (Option 2): Build from source

Prerequisites:

* [Rust](https://www.rust-lang.org/learn/get-started)
* [Git](https://git-scm.com/downloads)

Open a terminal in the desired folder and run the following commands:

```powershell
git clone https://github.com/sidit77/InputShareUSB.git
cd InputShareUSB
cargo build --bin inputshare-client --release
```

The binary will be in `/target/release/inputshare-client.exe`



#### Step 2: Configuration

* `Host`: The address of the raspberry pi that runs the sever. You can press the search button to automatically search in your local network.
* `Hotkey`: The hotkey that toggles input between the local and remote pc. The hotkey has two parts: the trigger key which triggers the swap and a variable amount of modifier keys which also have to be pressed for the trigger to work.
* `Blacklist`: All keys included in in this list will be ignored by the client.
* `Network Info`: When enabled the client will display the round-trip-time and packet loss to the server.
* `Mouse Speed`: changes the mouse speed of the remote device
* `network_send_rate` (config only): The number of packets per second that the client will send to the server while transmitting. Higher values mean lower latency and smoother mouse movement while lower values mean less network activity. Note that if the send rate is set to high it will flood the connection and cause massive delays / packet loss. Consider that the `mouse-tesselation-factor` option of the server has a similar effect and should be tuned in tandem.

The config is stored in `%appdata%/InputShare.ron`.

### Server

#### Step 1: Preparing the raspberry pi

The raspberry pi needs to be specifically configured to act like a usb device. To do so run the following command and restart the pi afterwards.

```bash
echo "dtoverlay=dwc2" | sudo tee -a /boot/config.txt
echo "dwc2" | sudo tee -a /etc/modules
echo "libcomposite" | sudo tee -a /etc/modules
```



#### Step 2: Select the correct version

Not every raspberry pi version uses the same architecture.

* For the raspberry pi zero pick `arm-unknown-linux-gnueabihf` (This will be the default for the following sections)
* For the raspberry pi 4 pick `armv7-unknown-linux-gnueabihf`



#### Step 3 (Option 1; Recommended): Download Prebuild Binary

You can download prebuild binaries for from the [Github Release Page](https://github.com/sidit77/InputShareUSB/releases).

Simply extract the binary and copy to the raspberry pi.



#### Step 3 (Option 2): Cross compile from source

Prerequisites:

* [cross](https://github.com/rust-embedded/cross)
* [Git](https://git-scm.com/downloads)

Open a terminal in the desired folder and run the following commands:

```powershell
git clone https://github.com/sidit77/InputShareUSB.git
cd InputShareUSB
cross build --bin inputshare-server --target=arm-unknown-linux-gnueabihf --release
```

The binary will be in `/target/arm-unknown-linux-gnueabihf/release/inputshare-server`.



#### Step 3 (Option 3): Build from source on the raspberry pi

**Warning: this can take very long (~20m) if you are use a raspberry pi zero**

Prerequisites (have to be install on the pi):

* *Rust* (`curl https://sh.rustup.rs -sSf | sh`)
* *Git* (`sudo apt install git`)

Run the following commands:

```bash
git clone https://github.com/sidit77/InputShareUSB.git
cd InputShareUSB
cargo build --bin inputshare-server --release
```

The binary will be in `/target/release/inputshare-server`



### Step 4: Running

To start the server simply run:

````
sudo ./inputshare-server
````

The sever can be configured using command line arguments (Run `./inputshare-sever -h` for more infomation).



### Step 5 (Optional): Creating a systemd service

You might want to automatically start the sever whenever the raspberry pi start. To do this copy `inputshare-server` to `/usr/bin/` and  create a new file `inputshare_server.service` in `/lib/systemd/system/`:

```bash
sudo cp inputshare-server /usr/bin/
sudo nano /lib/systemd/system/inputshare_server.service
```

and add the following content:

```bash
[Unit]
Description=Enables the InputShare server
After=multi-user.target

[Service]
Type=simple
ExecStart=/usr/bin/inputshare-server
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Now you can enable your new server using:

```bash
sudo systemctl enable inputshare_server.service
```



## License

MIT License

