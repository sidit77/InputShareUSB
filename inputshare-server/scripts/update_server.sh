#!/bin/bash

# Test if is Root
if [[ $(id -u) -ne 0 ]] ; then echo "Please run as root" ; exit 1 ; fi

cargo build --release

cp ../../target/release/inputshare-server /usr/bin/
chmod +x /usr/bin/inputshare-server