#!/usr/bin/env python3
"""
Hardware Interaction Training Data — serial devices, USB, HID, keystroke injection.
Teaches PlausiDen AI to interact with physical devices through various interfaces.
"""

import json
from pathlib import Path

DEFAULT_OUTPUT = "/home/user/LFI-data/hardware_training.jsonl"

TRAINING_DATA = [
    # === SERIAL DEVICE CONNECTIONS ===
    ("How do I connect to a serial device through USB?",
     "```bash\n# 1. Identify the device\ndmesg | tail -20  # check recent kernel messages after plugging in\nls /dev/ttyUSB* /dev/ttyACM*  # common serial device paths\n\n# 2. Check device info\nudevadm info -a /dev/ttyUSB0\n\n# 3. Connect with screen\nscreen /dev/ttyUSB0 115200  # 115200 is most common baud rate\n# Exit screen: Ctrl+A then K\n\n# 4. Or use minicom\nminicom -D /dev/ttyUSB0 -b 115200\n\n# 5. Or use picocom (simplest)\npicocom -b 115200 /dev/ttyUSB0\n# Exit: Ctrl+A then Ctrl+X\n\n# 6. If permission denied:\nsudo usermod -a -G dialout $USER\n# Then logout and login again\n\n# Common baud rates to try: 9600, 19200, 38400, 57600, 115200\n```",
     "serial_devices"),

    ("How do I identify an unknown USB device and find its serial protocol?",
     "```bash\n# 1. Physical identification\nlsusb  # list USB devices with vendor:product IDs\nlsusb -v -d VENDOR:PRODUCT  # detailed info\n\n# 2. Kernel driver loaded\ndmesg | grep -i 'usb\\|tty\\|serial\\|hid'\n\n# 3. Check udev rules\nudevadm info -a -n /dev/ttyUSB0 | grep -E 'ATTR|ID'\n\n# 4. Try auto-detection of baud rate\n# Use baudrate.py from devttys0:\npython3 baudrate.py -p /dev/ttyUSB0\n\n# 5. Manual baud rate detection\nfor baud in 9600 19200 38400 57600 115200; do\n  echo \"Trying $baud...\"\n  timeout 3 stty -F /dev/ttyUSB0 $baud\n  timeout 3 cat /dev/ttyUSB0 | xxd | head -5\n  echo '---'\ndone\n\n# 6. Check if it speaks a known protocol\n# AT commands (modems/cellular):\necho 'AT' > /dev/ttyUSB0\ncat /dev/ttyUSB0  # should see 'OK'\n\n# NMEA (GPS):\ncat /dev/ttyUSB0 | grep -E '^\\$G'  # GPS sentences start with $G\n```",
     "serial_devices"),

    ("How do I capture and analyze serial communication?",
     "```bash\n# 1. Using socat to log serial traffic\nsocat -v /dev/ttyUSB0,b115200 -  # interactive with hex dump\n\n# 2. Log to file\nsocat /dev/ttyUSB0,b115200,raw,echo=0 - | tee serial_capture.log\n\n# 3. Using interceptty for MITM serial logging\ninterceptty /dev/ttyUSB0 /tmp/fake_serial -s 'speed 115200'\n# Then connect your application to /tmp/fake_serial\n\n# 4. Binary capture with timestamps\npython3 -c \"\nimport serial, time\nser = serial.Serial('/dev/ttyUSB0', 115200, timeout=1)\nwith open('capture.bin', 'wb') as f:\n    while True:\n        data = ser.read(1024)\n        if data:\n            ts = time.time()\n            f.write(f'{ts}: '.encode() + data + b'\\n')\n            print(f'[{ts}] {data.hex()}')\n\"\n\n# 5. Analyze protocol patterns\nxxd capture.bin | head -50\npython3 -c \"data=open('capture.bin','rb').read(); print(f'Bytes: {len(data)}, Unique: {len(set(data))}')\"\n```",
     "serial_devices"),

    ("How do I interact with I2C and SPI devices on Linux?",
     "```bash\n# I2C\n# Install tools\nsudo apt install i2c-tools\n\n# Detect I2C buses\ni2cdetect -l\n\n# Scan for devices on bus 1\ni2cdetect -y 1\n\n# Read a byte from device 0x48, register 0x00\ni2cget -y 1 0x48 0x00\n\n# Write a byte\ni2cset -y 1 0x48 0x01 0xFF\n\n# Dump all registers\ni2cdump -y 1 0x48\n\n# SPI\n# Enable SPI in kernel\nsudo modprobe spidev\n\n# Python SPI interaction\npython3 -c \"\nimport spidev\nspi = spidev.SpiDev()\nspi.open(0, 0)  # bus 0, device 0\nspi.max_speed_hz = 1000000\nresult = spi.xfer2([0x01, 0x80, 0x00])  # send 3 bytes, read response\nprint(f'Response: {result}')\nspi.close()\n\"\n\n# GPIO (Raspberry Pi / SBCs)\nsudo apt install gpiod\ngpiodetect  # list GPIO chips\ngpioinfo    # show all GPIO lines\ngpioset 0 17=1  # set GPIO 17 high\ngpioget 0 18    # read GPIO 18\n```",
     "serial_devices"),

    ("How do I use Python to communicate with a serial device?",
     "```python\nimport serial\nimport time\n\n# Open connection\nser = serial.Serial(\n    port='/dev/ttyUSB0',\n    baudrate=115200,\n    bytesize=serial.EIGHTBITS,\n    parity=serial.PARITY_NONE,\n    stopbits=serial.STOPBITS_ONE,\n    timeout=1,\n    xonxoff=False,\n    rtscts=False,\n)\n\n# Send command\nser.write(b'AT\\r\\n')\ntime.sleep(0.5)\n\n# Read response\nresponse = ser.read(ser.in_waiting or 1)\nprint(f'Response: {response}')\n\n# Read line by line\nwhile True:\n    line = ser.readline().decode('utf-8', errors='replace').strip()\n    if line:\n        print(f'> {line}')\n\n# Binary protocol example\ndef send_packet(ser, cmd, data):\n    \"\"\"Send a framed packet: [START][LEN][CMD][DATA][CHECKSUM]\"\"\"\n    payload = bytes([cmd]) + data\n    checksum = sum(payload) & 0xFF\n    packet = bytes([0xAA, len(payload)]) + payload + bytes([checksum])\n    ser.write(packet)\n    return ser.read(64)  # read response\n\nser.close()\n```\n\nInstall: `pip3 install pyserial`",
     "serial_devices"),

    ("How do I set up a USB device to auto-configure on connect?",
     "Using udev rules:\n\n```bash\n# 1. Find device attributes\nudevadm info -a /dev/ttyUSB0 | grep -E 'ATTR{idVendor|idProduct|serial}'\n\n# 2. Create udev rule\nsudo cat > /etc/udev/rules.d/99-mydevice.rules <<'EOF'\n# Arduino Mega\nSUBSYSTEM==\"tty\", ATTRS{idVendor}==\"2341\", ATTRS{idProduct}==\"0042\", \\\n    SYMLINK+=\"arduino_mega\", MODE=\"0666\", \\\n    RUN+=\"/usr/local/bin/on_arduino_connect.sh\"\n\n# FTDI USB-Serial adapter\nSUBSYSTEM==\"tty\", ATTRS{idVendor}==\"0403\", ATTRS{idProduct}==\"6001\", \\\n    SYMLINK+=\"ftdi_serial\", MODE=\"0666\"\n\n# Any USB serial device\nSUBSYSTEM==\"tty\", ATTRS{idVendor}==\"*\", \\\n    RUN+=\"/usr/local/bin/serial_auto_detect.sh %k\"\nEOF\n\n# 3. Create handler script\ncat > /usr/local/bin/on_arduino_connect.sh <<'EOF'\n#!/bin/bash\nlogger \"Arduino connected on $DEVNAME\"\n# Start monitoring service, send notification, etc.\nEOF\nchmod +x /usr/local/bin/on_arduino_connect.sh\n\n# 4. Reload rules\nsudo udevadm control --reload-rules\nsudo udevadm trigger\n```",
     "serial_devices"),

    ("How do I use Ethernet for device communication (raw sockets)?",
     "```python\nimport socket\nimport struct\n\n# Raw Ethernet socket (requires root)\nsock = socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.ntohs(0x0003))\nsock.bind(('eth0', 0))\n\n# Receive raw frames\nwhile True:\n    frame, addr = sock.recvfrom(65535)\n    dst_mac = ':'.join(f'{b:02x}' for b in frame[0:6])\n    src_mac = ':'.join(f'{b:02x}' for b in frame[6:12])\n    ethertype = struct.unpack('!H', frame[12:14])[0]\n    print(f'{src_mac} -> {dst_mac} (0x{ethertype:04x}) {len(frame)} bytes')\n```\n\nFor device discovery on local Ethernet:\n```bash\n# ARP scan to find devices\nsudo arp-scan --localnet\n\n# Or with nmap\nnmap -sn 192.168.1.0/24\n\n# mDNS/Bonjour discovery\navahi-browse -alrt\n\n# LLDP discovery (network switches)\nsudo apt install lldpd\nlldpcli show neighbors\n```",
     "serial_devices"),

    # === HID (Human Interface Devices) ===
    ("How do I interact with USB HID devices on Linux?",
     "```bash\n# List HID devices\nls /dev/hidraw*\ncat /sys/class/hidraw/hidraw0/device/uevent\n\n# Read HID reports\nsudo cat /dev/hidraw0 | xxd\n\n# Python HID interaction\npip3 install hidapi\n\npython3 -c \"\nimport hid\n\n# List all HID devices\nfor d in hid.enumerate():\n    print(f\\\"{d['vendor_id']:04x}:{d['product_id']:04x} {d['product_string']}\\\")\n\n# Open specific device\nh = hid.device()\nh.open(0x1234, 0x5678)  # vendor_id, product_id\n\n# Read data\ndata = h.read(64, timeout_ms=1000)\nprint(f'Received: {data}')\n\n# Write data (first byte is report ID)\nh.write([0x00, 0x01, 0x02, 0x03])\n\nh.close()\n\"\n\n# Monitor USB HID events\nsudo apt install evtest\nsudo evtest  # lists input devices, select one\n\n# usbhid-dump for raw reports\nsudo apt install usbutils\nsudo usbhid-dump -d VENDOR:PRODUCT -e all\n```",
     "hid_devices"),

    ("How do I monitor keyboard and mouse input events on Linux?",
     "```bash\n# List input devices\ncat /proc/bus/input/devices\nls /dev/input/event*\n\n# Monitor specific device with evtest\nsudo evtest /dev/input/event0\n\n# Monitor all keyboard events\nsudo apt install evtest\nsudo evtest --grab /dev/input/by-id/*-kbd\n\n# Python input monitoring\npip3 install evdev\n\npython3 -c \"\nfrom evdev import InputDevice, categorize, ecodes\n\ndev = InputDevice('/dev/input/event0')\nprint(f'Device: {dev.name}')\n\nfor event in dev.read_loop():\n    if event.type == ecodes.EV_KEY:\n        key_event = categorize(event)\n        if key_event.keystate == key_event.key_down:\n            print(f'Key pressed: {key_event.keycode}')\n\"\n\n# Read mouse movement\npython3 -c \"\nfrom evdev import InputDevice, ecodes\ndev = InputDevice('/dev/input/event1')  # mouse\nfor event in dev.read_loop():\n    if event.type == ecodes.EV_REL:\n        if event.code == ecodes.REL_X:\n            print(f'Mouse X: {event.value}')\n        elif event.code == ecodes.REL_Y:\n            print(f'Mouse Y: {event.value}')\n\"\n```",
     "hid_devices"),

    ("How do I create a USB HID device emulator (like a Rubber Ducky)?",
     "Using Linux USB Gadget (for authorized testing/development):\n\n```bash\n# On a device that supports USB gadget (Raspberry Pi Zero, etc.)\nsudo modprobe libcomposite\n\n# Create HID gadget\ncd /sys/kernel/config/usb_gadget/\nmkdir keyboard_gadget && cd keyboard_gadget\n\necho 0x1d6b > idVendor   # Linux Foundation\necho 0x0104 > idProduct  # Multifunction Composite Gadget\necho 0x0100 > bcdDevice\necho 0x0200 > bcdUSB\n\nmkdir -p strings/0x409\necho 'Test Device' > strings/0x409/manufacturer\necho 'HID Keyboard' > strings/0x409/product\n\nmkdir -p configs/c.1/strings/0x409\necho 'Config 1' > configs/c.1/strings/0x409/configuration\n\nmkdir -p functions/hid.usb0\necho 1 > functions/hid.usb0/protocol  # keyboard\necho 1 > functions/hid.usb0/subclass\necho 8 > functions/hid.usb0/report_length\n\n# HID report descriptor for keyboard\necho -ne '\\x05\\x01\\x09\\x06\\xa1\\x01\\x05\\x07\\x19\\xe0\\x29\\xe7\\x15\\x00\\x25\\x01\\x75\\x01\\x95\\x08\\x81\\x02\\x95\\x01\\x75\\x08\\x81\\x01\\x95\\x05\\x75\\x01\\x05\\x08\\x19\\x01\\x29\\x05\\x91\\x02\\x95\\x01\\x75\\x03\\x91\\x01\\x95\\x06\\x75\\x08\\x15\\x00\\x25\\x65\\x05\\x07\\x19\\x00\\x29\\x65\\x81\\x00\\xc0' > functions/hid.usb0/report_desc\n\nln -s functions/hid.usb0 configs/c.1/\nls /sys/class/udc > UDC\n```\n\n**Python to send keystrokes:**\n```python\ndef send_key(dev, key, modifier=0):\n    report = bytes([modifier, 0, key, 0, 0, 0, 0, 0])\n    with open(dev, 'wb') as f:\n        f.write(report)\n        f.write(bytes(8))  # release\n\nsend_key('/dev/hidg0', 0x04)  # 'a'\n```\n\n**This is for authorized device testing and development only.**",
     "hid_devices"),

    ("How do I detect and defend against keystroke injection attacks?",
     "**Detection methods:**\n\n```bash\n# 1. Monitor USB device connections\nsudo udevadm monitor --subsystem-match=usb\n\n# 2. USBGuard — whitelist-based USB device policy\nsudo apt install usbguard\nsudo usbguard generate-policy > /etc/usbguard/rules.conf\n# Edit to allow only known devices\nsudo systemctl enable usbguard\nsudo systemctl start usbguard\n\n# List connected devices\nsudo usbguard list-devices\n\n# Allow a new device\nsudo usbguard allow-device <id>\n\n# Block a device\nsudo usbguard block-device <id>\n\n# 3. Monitor typing speed (rubber duckies type inhumanly fast)\npython3 -c \"\nfrom evdev import InputDevice, ecodes\nimport time\n\ndev = InputDevice('/dev/input/event0')\nlast_key_time = 0\nfor event in dev.read_loop():\n    if event.type == ecodes.EV_KEY and event.value == 1:  # key down\n        now = time.time()\n        delta = now - last_key_time\n        if delta < 0.01 and last_key_time > 0:  # <10ms between keys\n            print(f'WARNING: Suspiciously fast typing ({delta*1000:.1f}ms)')\n        last_key_time = now\n\"\n\n# 4. Disable USB HID temporarily\necho 0 | sudo tee /sys/bus/usb/drivers/usbhid/*/authorized\n```\n\n**Key defenses:**\n- USBGuard with device whitelist\n- Monitor for new HID devices appearing unexpectedly\n- Alert on superhuman typing speeds\n- Physical port locks/blockers\n- BIOS-level USB port control",
     "hid_devices"),

    ("How do I use BadUSB/Rubber Ducky payloads for security testing?",
     "**DuckyScript syntax (for authorized penetration testing):**\n\n```\nREM Open terminal and run a command\nDELAY 1000\nGUI r\nDELAY 500\nSTRING cmd\nENTER\nDELAY 1000\nSTRING whoami\nENTER\n```\n\n**DuckyScript commands:**\n- `STRING text` — type text\n- `ENTER` — press Enter\n- `DELAY ms` — wait\n- `GUI` — Windows/Super key\n- `ALT`, `CTRL`, `SHIFT` — modifier keys\n- `TAB`, `ESC`, `SPACE` — special keys\n\n**Linux equivalent with `xdotool`** (for testing):\n```bash\n# Simulate keyboard input\nxdotool type 'whoami'\nxdotool key Return\nxdotool key super  # open application menu\n\n# Simulate mouse\nxdotool mousemove 100 200\nxdotool click 1  # left click\n```\n\n**Defense testing checklist:**\n1. Does USBGuard block unknown HID devices?\n2. Does the system alert on new USB insertions?\n3. Are USB ports disabled in BIOS for kiosks?\n4. Does endpoint detection catch the payload execution?\n\n**Always have written authorization. Document everything.**",
     "hid_devices"),

    # === USB Device Analysis ===
    ("How do I analyze USB traffic with Wireshark?",
     "```bash\n# Load USB monitor kernel module\nsudo modprobe usbmon\n\n# Find your USB bus number\nlsusb  # note the Bus number\n\n# Capture with tshark\nsudo tshark -i usbmon1  # bus 1\n\n# In Wireshark: select usbmonX interface\n# Filter for specific device:\n# usb.addr == \"1.3.0\"  (bus.device.endpoint)\n\n# Capture to file\nsudo tshark -i usbmon1 -w usb_capture.pcap -c 1000\n\n# Filter HID data\ntshark -r usb_capture.pcap -Y 'usb.transfer_type == 0x01' -T fields -e usb.capdata\n\n# Decode HID reports\npython3 -c \"\nimport struct\n# Example: parse keyboard HID report\nreport = bytes.fromhex('02000400000000')  # Shift+A\nmodifier = report[0]\nkeys = report[2:8]\nprint(f'Modifier: {modifier:08b}')\nfor k in keys:\n    if k: print(f'Key code: 0x{k:02x}')\n\"\n```\n\nUseful Wireshark filters:\n- `usb.bmRequestType == 0x21` — HID SET_REPORT\n- `usb.bDescriptorType == 0x22` — HID Report Descriptor\n- `usb.transfer_type == 0x01` — Interrupt transfers (HID data)",
     "hid_devices"),

    ("How do I connect to and interact with Bluetooth devices?",
     "```bash\n# Install tools\nsudo apt install bluetooth bluez bluez-tools\n\n# Start Bluetooth service\nsudo systemctl start bluetooth\n\n# Scan for devices\nhcitool scan\n# Or newer:\nbluetoothctl\n[bluetooth]# scan on\n[bluetooth]# devices  # list found devices\n\n# Pair and connect\n[bluetooth]# pair XX:XX:XX:XX:XX:XX\n[bluetooth]# connect XX:XX:XX:XX:XX:XX\n[bluetooth]# trust XX:XX:XX:XX:XX:XX\n\n# BLE (Bluetooth Low Energy) scanning\nsudo hcitool lescan\n\n# Read BLE characteristics\nsudo gatttool -b XX:XX:XX:XX:XX:XX --interactive\n[XX:XX:XX:XX:XX:XX][LE]> connect\n[XX:XX:XX:XX:XX:XX][LE]> primary  # list services\n[XX:XX:XX:XX:XX:XX][LE]> characteristics  # list characteristics\n[XX:XX:XX:XX:XX:XX][LE]> char-read-hnd 0x0025  # read a characteristic\n\n# Python BLE\npip3 install bleak\npython3 -c \"\nimport asyncio\nfrom bleak import BleakScanner\nasync def main():\n    devices = await BleakScanner.discover()\n    for d in devices:\n        print(f'{d.address}: {d.name}')\nasyncio.run(main())\n\"\n```",
     "serial_devices"),
]


def main():
    output_path = DEFAULT_OUTPUT

    Path(output_path).parent.mkdir(parents=True, exist_ok=True)

    print(f"=== Hardware Interaction Training Data ===")
    counts = {}
    with open(output_path, "w") as f:
        for instruction, output, domain in TRAINING_DATA:
            pair = {
                "instruction": instruction,
                "input": "",
                "output": output,
                "source": "hardware_training",
                "domain": domain,
            }
            f.write(json.dumps(pair) + "\n")
            counts[domain] = counts.get(domain, 0) + 1

    total = sum(counts.values())
    print(f"Domain breakdown:")
    for domain, count in sorted(counts.items()):
        print(f"  {domain}: {count}")
    print(f"\n=== Done: {total} hardware examples written ===")


if __name__ == "__main__":
    main()
