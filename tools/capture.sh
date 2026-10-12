#!/usr/bin/env bash
# Capture all USB traffic on the MasterHUB's bus via usbmon (needs root).
# Usage: sudo tools/capture.sh <name>   -> captures/<name>.pcapng  (Ctrl+C to stop)
set -euo pipefail
name=${1:?capture name}
bus=$(for d in /sys/bus/usb/devices/*; do [[ -f $d/idVendor && $(cat $d/idVendor) == 2516 ]] && cat $d/busnum && break; done)
: "${bus:?MasterHUB not found on host (is it attached to the VM? bus number still applies — set BUS=)}"
bus=${BUS:-$bus}
modprobe usbmon
out="$(dirname "$0")/../captures/$name.pcapng"
echo "Capturing usbmon$bus -> $out"
exec dumpcap -i "usbmon$bus" -w "$out"
