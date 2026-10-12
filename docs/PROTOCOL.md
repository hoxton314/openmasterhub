# MasterHUB USB protocol notes

Status: **in progress**, descriptor-level facts are confirmed and everything else is a hypothesis until captured.

## Topology

The MasterHUB is not a single device. The base contains a chain of Prolific
USB 2.0 hubs (`067b:2586`), and **every module enumerates as its own USB HID device**.

| Module (iProduct)    | VID:PID     | Serial prefix | Speed | OUT EP size | bInterval |
|----------------------|-------------|---------------|-------|-------------|-----------|
| Base, 4x6            | `2516:01f6` | `MHBS01`      | FS    | 64          | 3 ms      |
| 15xKey w/ IPS, 4x3   | `2516:01fe` | `MHKD15`      | HS    | 320         | 1 (125µs) |
| 5xFader, 4x2         | `2516:0200` | `MHFD05`      | FS    | 64          | 3 ms      |
| 2xRoller, 4x1        | `2516:0204` | `MHRR02`      | FS    | 64          | 3 ms      |

The trailing "4xN" in the product string looks like the module's footprint in grid units.

Each device exposes one HID interface (class 3, no boot protocol) with EP1 IN plus EP2 OUT (interrupt).

## HID report descriptor (common skeleton)

Usage Page `0x0C` (Consumer), Usage `0x01`, then everything is vendor usage `0xFF00`,
8-bit fields with logical range 0..255:

| Report ID | Type    | Payload size (excl. ID) | Present on                                 |
|-----------|---------|-------------------------|--------------------------------------------|
| `0x02`    | Output  | 63 (**319 on Key/IPS**) | all                                        |
| `0x01`    | Input   | 63                      | all                                        |
| `0x03`    | Feature | 31                      | all                                        |
| `0x2D`–`0x32` | Feature | 31                  | Base, Fader, Roller                        |
| `0x33`    | Feature | 31                          | Fader                                      |
| `0x34`, `0x35` | Feature | 31                 | Base (`34`,`35`), Fader (`34`)             |
| `0x36`    | Feature | 31                          | Base, Fader, Roller                        |
| `0x37`–`0x3E` | Feature | 31                  | Key/IPS                                    |

Hypotheses:
- Output `0x02` carries commands such as LED, config, and the IPS image stream (319 bytes for the Key module).
- Input `0x01` carries events such as key presses, fader positions, and roller deltas.
- The feature reports are probably config/info pages (firmware version, calibration, etc.).

Raw descriptors are saved in `docs/descriptors/`.

## Vendor software (static analysis sources)

- **MasterHUB app** v1.0.0.113 (`linkto.cm/dl-masterhub`) drives the hardware. It is Electron + Vue (source maps shipped, so the full renderer source can be recovered) plus the native addon `stackaddon.node`, which wraps per-module SDK DLLs (`CoolTEK_SDK_CM_<model>.dll`, which embed hidapi).
- **MasterCTRL** only lists the MasterHUB as a device and has no protocol logic for it.

| SDK family | PIDs (prod / alt)  | Module                  |
|------------|--------------------|-------------------------|
| M0         | `0x01f6` / `0x9999` | Base                    |
| M0         | `0x0200` / `0x9991` | Fader (Slider)          |
| M0         | `0x0202` / `0x9996` | Knob                    |
| M0         | `0x0204` / `0x9995` | Roller (Wheel)          |
| M4315      | `0x01fe` / `0x9997` | 15xKey w/ IPS           |
| M131E      | `0x0206` / `0x9994` | display module (TBD)    |
| M3700/M3704/M4300/M4600/M5000 | `0x9988` `0x9989` `0x9998` `0x9993` `0x9992` | other/legacy modules |

Detailed specs:
- [`protocol-m0.md`](protocol-m0.md): Base, Fader, Knob, Roller
- [`protocol-m4315.md`](protocol-m4315.md): 15-key IPS keypad
- [`app-architecture.md`](app-architecture.md): topology detection, event loop, actions, profiles

### Confirmed so far (M0)
- Feature `0x31` ("Get_Device_Infos") is a 32-byte get; byte[2] is the device info.
- Feature `0x2D`/`0x2E`/`0x2F` give FW info for type 0/1/2. The layout is `[id][?][ver:4][string:11]`.
- Input report `0x01` uses `[0x01][type][len:u16le][payload...]` (payload at offset 4, length at offset 2).

## Method

1. Pass the module devices into the Windows VM (`tools/vm/*.xml`, libvirt USB hostdev).
2. Capture on the host with `sudo tools/capture.sh <name>` (usbmon on the hub's bus) while performing exactly one action in the MasterHUB app.
3. Diff the captures and note the findings here.
4. Statically analyse the MasterHUB installer to cross-check the findings.

## Verified on hardware (Linux hidraw, 2026-10-12)

**15xKey IPS (`01fe`):**
- **Key reporting needs software mode.** SET feature `03 3A 01` stops the built-in logo animation. After that, every press and every release sends input report `01 34 11 00 02 0f s0..s14`, a full state snapshot with 1 = pressed. In hardware mode (the default after power-up) keys send nothing to the host.
- Key index runs 0..14, row-major from the top-left: 0 is top-left, 4 top-right, 14 bottom-right.
- Some feature GETs return 0 bytes before software mode is entered. After it, they all work:
  - `0x37`: bootloader version, `[37 0f crc:4 "02.00.00.00"]`
  - `0x38`/`0x39`: firmware version, `"02.00.01.00"`
  - `0x3A`: serial number, `[3a 14 "MHKD15AA431251602222"]`
  - `0x3B`: device info, `3b 0b 14 03 05 0f e0 01 10 01 01 48 48`. Byte 3 is rows = 3, byte 4 cols = 5, byte 5 keys = 15. Bytes 6..7 give the screen width, 480 (u16 LE), and bytes 8..9 the height, 272. Bytes 11 and 12 are the key image width and height, 72×72.
  - `0x3C`: backlight, `[3c 01 <0..100>]`

**Fader (`0200`):** input `01 2a 07 00 02 05 f0 f1 f2 f3 f4`.
- Index 0 is the **top** fader and index 4 the bottom one.
- A position reads 0 at the **left** end and 100 at the right.
- It reports continuously while moving, with no host setup needed.

**Roller (`0204`):** input `01 2a 04 00 01 02 R L`.
- Byte 6 is the **right** roller and byte 7 the **left** one.
- `0` means one click rolled away from the user (toward the keys), `1` one click toward the user, and `0xff` no movement.
- It sends one report per click.

**Base (`01f6`):** input `01 2b 01 00 01` (orientation, 1 = horizontal), sent repeatedly about every 0.5 s.

**M0 firmware info** (feature `0x2D` = bootloader, `0x2E`/`0x2F` = application): `[id 0f crc:4 "vv.vv.vv.vv"]`.
- Base: 02.00.04.00
- Fader: 01.00.05.00
- Roller: 01.00.03.01

Feature `0x31` returns `31 14 00…` and feature `0x36` returns `36 10 <5 bytes> 00 00 00 <same 5 bytes>`; the meaning of both is still unknown.

**Key images (`01fe`), confirmed:** output `02 36 seq:u16le len:u16le key last data[≤312]`, zero-padded to 320 bytes, carrying a 72×72 baseline JPEG (PIL, quality 85, about 1.7 KB = 6 packets).
- The image displays upright with no rotation or mirroring.
- Writing all 15 keys back-to-back took 45 ms on the host with no glitches. That covers sending only; the screen update time wasn't measured.
- The module must be in software mode first.

**LEDs (M0), confirmed.** Each command is a 64-byte output report sent to the module that owns the LED:
- `02 2C group idx mode save`: mode 1 is static.
- `02 2D group idx R G B save`: colour.
- `02 2E group idx brightness`: brightness, 1..100.
- Fader LEDs sit along each fader track: group 1, idx 0 (top) to 4 (bottom), and `0xFF` addresses all of them.
- The Base has a single LED zone: group 0, idx 0.
- LEDs that haven't been set keep running their default colour-cycle effect.
- In software mode, a colour change alone takes effect immediately.

**Important: SET commands on display modules are applied only after GET feature `0x03`.** A SET feature `03 <cmd> …` on the M4315 or M131E is acknowledged on the bus but **does nothing** until the host reads feature report `0x03` back (it returns `03 00 …`). The vendor hidapi wrapper does this implicitly. `tools/mhprobe.py set_feature()` now always reads back.

**Knob (`0202`, "3xKnob, 2x2"):** input `01 2a 05 00 <sub> 03 k0 k1 k2`.
- The three knobs form a triangle. **k0 = bottom-left, k1 = bottom-right, k2 = top** (the apex).
- `sub 00` is rotation, one report per detent: `01` = clockwise, `00` = counter-clockwise, `ff` = no movement.
- `sub 03` is button state: `1` = pressed, `0` = released. A full state is sent on each change.
- FW 01.00.05.00. **LEDs confirmed:** group 2, idx 0 = bottom-left, 1 = bottom-right, 2 = top (same order as the input). They default to colour cycling until set (`02 2C 2 idx 1 0` static, `02 2D 2 idx R G B 0`, `02 2E 2 idx 100`).

**Dial (`0206`, M131E, "1xDial w/ IPS, 2x2"):**
- **Software mode:** SET `03 49 01` and then GET `0x03`. `03 4A` (a +0x10 port from the M4315) is wrong.
- **Feature GETs** (they work once in software mode):
  - `0x47`: bootloader, "01.00.00.00"
  - `0x48`: firmware, "01.00.04.00"
  - `0x4A`: serial number
  - `0x4B`: device info, `4b 0b 14 00 00 00 f0 00 f0 00 01 f0 f0`. That is 0 keys, a 240×240 screen and a 240×240 image.
  - `0x4C`: backlight, 100
- **Solid colour:** SET `03 46 R G B` (+ GET 0x03). Confirmed.
- **Image:** output `02 45 seq:u16le len:u16le 00 last data[≤312]`, padded to 320 bytes, carrying a 240×240 JPEG. An 11 KB image took 35 packets. It displays upright and isn't mirrored.
- **Input** is `01 44 03 00 d0 01 d2`:
  - `d0 = 03` is rotation: `d2 0` = clockwise, `1` = counter-clockwise, about 36 steps per revolution.
  - `d0 = 04` is the button: `d2 1` = down, `0` = up.
