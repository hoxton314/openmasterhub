#!/usr/bin/env bash
# Re-attach the MasterHUB modules to the running win11 VM. libvirt passes USB devices through
# only once: after the hub is replugged they come back to Linux until re-attached.
set -u
here=$(cd "$(dirname "$0")" && pwd)
vm=${1:-win11}
for xml in "$here"/masterhub-*.xml; do
  pid=$(sed -n "s/.*product id='0x\([0-9a-f]*\)'.*/\1/p" "$xml")
  lsusb -d "2516:$pid" > /dev/null 2>&1 || continue   # module not plugged in
  virsh -c qemu:///system detach-device "$vm" "$xml" --live > /dev/null 2>&1
  virsh -c qemu:///system attach-device "$vm" "$xml" --live > /dev/null && echo "attached 2516:$pid"
done
