#!/bin/bash
# NETWORK RESCUE — Restore workstation network if Pineapple operation broke it
# Run from a local TTY (Ctrl+Alt+F2) if you can't reach a terminal
# NO DEPENDENCIES — works without internet, without Rust, without anything

set -x  # show every command

# Stop anything that might be holding wlan0
sudo airmon-ng stop wlan0mon 2>/dev/null
sudo airmon-ng stop wlan1mon 2>/dev/null
sudo killall airodump-ng 2>/dev/null
sudo killall hcxdumptool 2>/dev/null
sudo killall tcpdump 2>/dev/null

# Force wlan0 back to managed mode
sudo ip link set wlan0 down 2>/dev/null
sudo iw dev wlan0 set type managed 2>/dev/null
sudo ip link set wlan0 up

# Restart the network stack
sudo systemctl restart wpa_supplicant
sudo systemctl restart NetworkManager
sleep 5

# Reconnect
sudo nmcli device connect wlan0

# Verify
ip link show wlan0
ip route show
ping -c 3 8.8.8.8

echo ""
echo "If this didn't work, reboot. Network is fragile but not broken."
