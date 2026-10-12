#!/usr/bin/env python3
"""Minimal usbmon binary-interface reader (no wireshark needed).

  usbmon_dump.py BUS [DEVNUM ...] [--seconds N]

Prints one line per URB event: time, S/C/E, transfer type, dev, endpoint, status, length, data.
Needs read access to /dev/usbmonBUS (modprobe usbmon; setfacl -m u:$USER:r /dev/usbmonN).
"""
import argparse
import os
import select
import struct
import time

HDR = struct.Struct("<QBBBBHbbqiiII8s")  # 48-byte mon_bin_hdr
XFER = {0: "ISO", 1: "INT", 2: "CTL", 3: "BLK"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("bus", type=int)
    ap.add_argument("dev", type=int, nargs="*")
    ap.add_argument("--seconds", type=float, default=0)
    a = ap.parse_args()
    fd = os.open(f"/dev/usbmon{a.bus}", os.O_RDONLY)
    t0, end = time.time(), (time.time() + a.seconds if a.seconds else None)
    while end is None or time.time() < end:
        if not select.select([fd], [], [], 0.5)[0]:
            continue
        buf = os.read(fd, HDR.size + 4096)
        (_id, typ, xfer, ep, dev, _bus, _fs, _fd, sec, usec, status, length, cap, setup) = HDR.unpack_from(buf)
        if a.dev and dev not in a.dev:
            continue
        data = buf[HDR.size:HDR.size + cap]
        direction = "IN " if ep & 0x80 else "OUT"
        extra = f" setup={setup.hex()}" if xfer == 2 and typ == ord("S") else ""
        print(f"{sec + usec / 1e6 - t0:9.3f} {chr(typ)} {XFER.get(xfer, xfer)} dev{dev} ep{ep & 0x7f}{direction} "
              f"st={status} len={length}{extra} {data.hex(' ')}", flush=True)


if __name__ == "__main__":
    main()
