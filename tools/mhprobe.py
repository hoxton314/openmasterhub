#!/usr/bin/env python3
"""MasterHUB probe tool: talks to the modules via Linux hidraw, no dependencies.

  mhprobe.py list                  list modules and their hidraw nodes
  mhprobe.py info                  read the read-only feature reports of every module
  mhprobe.py monitor               dump decoded input reports (Ctrl+C to stop)
  mhprobe.py key-image KEY FILE    upload a 72x72 JPEG to a key of the 15xKey IPS module

Only non-destructive commands are implemented on purpose (no FW/serial/factory ops).
"""
import fcntl
import glob
import os
import select
import sys

VID = 0x2516
MODULES = {
    0x01F6: ("Base", "m0"), 0x9999: ("Base", "m0"),
    0x0200: ("Fader", "m0"), 0x9991: ("Fader", "m0"),
    0x0202: ("Knob", "m0"), 0x9996: ("Knob", "m0"),
    0x0204: ("Roller", "m0"), 0x9995: ("Roller", "m0"),
    0x01FE: ("15xKey IPS", "m4315"), 0x9997: ("15xKey IPS", "m4315"),
    0x0206: ("Dial", "m131e"), 0x9994: ("Dial", "m131e"),
}
# Read-only feature report IDs per SDK family (see docs/protocol-*.md)
FEATURES = {
    "m0": [0x2D, 0x2E, 0x2F, 0x31, 0x36],
    "m4315": [0x37, 0x38, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E],
    "m131e": [0x47, 0x48, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E],
}
FEATURE_LEN = 32  # report ID + 31 bytes


def HIDIOCGFEATURE(length):
    # _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x07, len)
    return (3 << 30) | (length << 16) | (ord("H") << 8) | 0x07


def HIDIOCSFEATURE(length):
    # _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len)
    return (3 << 30) | (length << 16) | (ord("H") << 8) | 0x06


def set_feature(fd, data):
    """SET feature report 0x03 command, then GET 0x03 back.

    The display modules (M4315/M131E) only apply a SET once report 0x03 has been read back.
    """
    buf = bytearray(bytes(data).ljust(FEATURE_LEN, b"\0"))
    fcntl.ioctl(fd, HIDIOCSFEATURE(len(buf)), buf, True)
    return get_feature(fd, 0x03)


def find_modules():
    found = []
    for node in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        uevent = open(os.path.join(node, "device/uevent")).read()
        hid_id = next(l for l in uevent.splitlines() if l.startswith("HID_ID="))
        _, vid, pid = hid_id.split("=")[1].split(":")
        vid, pid = int(vid, 16), int(pid, 16)
        if vid != VID:
            continue
        serial = next((l.split("=", 1)[1] for l in uevent.splitlines() if l.startswith("HID_UNIQ=")), "")
        name, family = MODULES.get(pid, (f"unknown {pid:04x}", None))
        found.append({"dev": "/dev/" + os.path.basename(node), "pid": pid, "name": name,
                      "family": family, "serial": serial})
    return found


def get_feature(fd, report_id):
    buf = bytearray(FEATURE_LEN)
    buf[0] = report_id
    n = fcntl.ioctl(fd, HIDIOCGFEATURE(len(buf)), buf, True)
    return bytes(buf[:n])


def hexs(b):
    return " ".join(f"{x:02x}" for x in b)


def cmd_list(mods):
    for m in mods:
        print(f"{m['dev']:14} {m['pid']:04x} {m['name']:14} {m['serial']}")


def cmd_info(mods):
    for m in mods:
        print(f"== {m['name']} ({m['dev']}, {m['serial']})")
        fd = os.open(m["dev"], os.O_RDWR)
        try:
            for rid in FEATURES.get(m["family"], []):
                try:
                    data = get_feature(fd, rid)
                    printable = "".join(chr(c) if 32 <= c < 127 else "." for c in data)
                    print(f"  feat {rid:02x}: {hexs(data)}  |{printable}|")
                except OSError as e:
                    print(f"  feat {rid:02x}: error {e}")
        finally:
            os.close(fd)


def decode(m, r):
    if len(r) < 4 or r[0] != 0x01:
        return ""
    kind, ln = r[1], r[2] | (r[3] << 8)
    p = r[4:4 + ln]
    if m["family"] == "m4315" and kind == 0x34 and len(p) >= 17:
        pressed = [i for i, s in enumerate(p[2:17]) if s]
        return f"keys pressed={pressed}"
    if m["name"] == "Fader" and kind == 0x2A and len(p) >= 7:
        return f"faders={list(p[2:7])}"
    if m["name"] == "Roller" and kind == 0x2A and len(p) >= 4:
        return f"roller right={p[2]:#x} left={p[3]:#x}"
    if m["name"] == "Knob" and kind == 0x2A and len(p) >= 5:
        what = "rotate" if p[0] == 0 else "buttons" if p[0] == 3 else f"sub{p[0]}"
        return f"knob {what} k0={p[2]:#x} k1={p[3]:#x} k2={p[4]:#x}"
    if m["family"] == "m131e" and kind == 0x44 and len(p) >= 3:
        if p[0] == 3:
            return f"dial {'CW' if p[2] == 0 else 'CCW'}"
        if p[0] == 4:
            return f"dial button {'down' if p[2] else 'up'}"
    if m["name"] == "Base" and kind == 0x2B and p:
        return f"base horizontal={p[0] == 1}"
    return f"kind={kind:#x} len={ln}"


def cmd_monitor(mods):
    fds = {os.open(m["dev"], os.O_RDONLY | os.O_NONBLOCK): m for m in mods}
    print("monitoring, Ctrl+C to stop")
    try:
        while True:
            for fd in select.select(list(fds), [], [])[0]:
                m = fds[fd]
                r = os.read(fd, 512)
                print(f"{m['name']:10} {hexs(r[:24])}{' ...' if len(r) > 24 else ''}  {decode(m, r)}", flush=True)
    except KeyboardInterrupt:
        pass


def cmd_key_image(mods, key, path):
    m = next((m for m in mods if m["family"] == "m4315"), None)
    if not m:
        sys.exit("no 15xKey IPS module found")
    data = open(path, "rb").read()
    if data[:2] != b"\xff\xd8":
        sys.exit("expected a JPEG (72x72)")
    chunk, seq = 312, 0
    fd = os.open(m["dev"], os.O_RDWR)
    try:
        set_feature(fd, [0x03, 0x3A, 0x01])  # software mode, needed for host-driven images
        for off in range(0, len(data), chunk):
            part = data[off:off + chunk]
            last = off + chunk >= len(data)
            pkt = bytes([0x02, 0x36, seq & 0xFF, seq >> 8, len(part) & 0xFF, len(part) >> 8, key, int(last)]) + part
            os.write(fd, pkt.ljust(320, b"\0"))
            seq += 1
    finally:
        os.close(fd)
    print(f"sent {len(data)} bytes in {seq} packets to key {key}")


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    mods = find_modules()
    if not mods:
        sys.exit("no MasterHUB modules found (attached to the VM?)")
    cmd = sys.argv[1]
    if cmd == "list":
        cmd_list(mods)
    elif cmd == "info":
        cmd_info(mods)
    elif cmd == "monitor":
        cmd_monitor(mods)
    elif cmd == "key-image" and len(sys.argv) == 4:
        cmd_key_image(mods, int(sys.argv[2]), sys.argv[3])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
