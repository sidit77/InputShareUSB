#!/bin/bash

# Test if is Root
if [[ $(id -u) -ne 0 ]] ; then echo "Please run as root" ; exit 1 ; fi

# Install service
cp inputshare_server.service /lib/systemd/system/

# Enable service
systemctl enable inputshare_server.service