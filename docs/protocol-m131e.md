# CM_M131E "1.3” Dial Encoder" module: wire protocol

Device: VID `0x2516`, PID `0x0206`. The add-on falls back to PID **`0x9994`** when `0x0206` fails to open
(`stackaddon.node.c:4953/4974`), which is probably the bootloader / recovery PID, as with M4315's `0x9997`.
The app calls the module "1.3” Dial Encoder" (`dialEncoder`, `knobx1`, model type `'m131'`). Its firmware file is
`CM_M131E_V<ver>_download.sys` (`background.pretty.js:48069`).

This document only records **how M131E differs** from [`protocol-m4315.md`](protocol-m4315.md). Anything not
mentioned here (transport conventions, SDK return codes, the 320-byte stale output buffer, ring-buffer
queueing, the BMP/JPEG validator, the CRC-32 used by animation flashing) is the same code and behaves as described there.

Sources: `decomp/CoolTEK_SDK_CM_M131E_V02.00.01.dll.c` (cited below as `M131E.c:<line>`), compared
function by function against `decomp/CoolTEK_SDK_CM_M4315_V02.00.00.dll.c`; `decomp/stackaddon.node.c`;
`background.pretty.js` (main process); `hub_src/src/App.vue`, `store/module/knobs.js`, `store/module/m131e.js`;
and `docs/descriptors/0206.bin` (a real descriptor dump).

Confidence: **High** = read directly from code or the descriptor. **Medium** = clear code, inferred meaning.
**Low** = guess. Nothing in this file has been tested on M131E hardware yet.

---

## 0. The "+0x10" rule is wrong for SET and Output commands

The earlier note said every M131E command byte is the M4315 byte + 0x10. That holds only for
**feature GET report IDs** and **input sub-types**. The M131E SDK has no per-button functions, so the
M4315 "button" slot is missing from both the SET and Output enumerations, and **every command after it
is +0x0F, not +0x10**. (High: each byte below is a literal in the decompile.)

| Function | M4315 | **M131E** | Shift | Evidence (M131E.c) |
|---|---|---|---|---|
| **Feature SET** `[03, sub, …]` (32 B) | | | | |
| Adjust_Backlight | `03 34 lvl` | **`03 44 lvl`** | +0x10 | 1585 `puVar1[1] = 0x44` |
| Set_Serial_Number (DANGEROUS) | `03 35` | **`03 45`** | +0x10 | 1522 `puVar1[1] = 0x45` |
| Display_Color_On_Screen | `03 36 R G B` | **`03 46 R G B`** | +0x10 | 2020 `puVar1[1] = 0x46` |
| Display_Color_On_Button | `03 37` | *(does not exist)* | | |
| Test_FW_Recovery (DANGEROUS) | `03 39` | **`03 48`** | **+0x0F** | 2236 `puVar1[1] = 0x48` |
| **Set_Sw_Mode** | `03 3A m` | **`03 49 m`** | **+0x0F** | 1554 `puVar1[1] = 0x49` |
| Save_Factory_Data (DANGEROUS) | `03 3C` | **`03 4B`** | **+0x0F** | 2323 `puVar1[1] = 0x4b` |
| Test_Factory_Reset (DANGEROUS) | `03 3D` | **`03 4C`** | **+0x0F** | 2265 `puVar1[1] = 0x4c` |
| Test_Animation_Recovery (DANGEROUS) | `03 3E` | **`03 4D`** | **+0x0F** | 2294 `puVar1[1] = 0x4d` |
| **Output** `[02, cmd, …]` (320 B) | | | | |
| Update_FW_by_type (DANGEROUS) | `02 34` | **`02 44`** | +0x10 | 1675 `puVar3[1] = 0x44` |
| Display_Picture_On_Screen | `02 35` | **`02 45`** | +0x10 | 1922 `local_90[1] = 0x45` |
| Display_Picture_On_Button | `02 36` | *(does not exist)* | | |
| Save_Screen_Picture (DANGEROUS) | `02 38` | **`02 47`** | **+0x0F** | 2398 `puVar1[1] = 0x47` |
| Get_Config_Data (b != 0) | `02 39` | **`02 48`** | **+0x0F** | 2357 |
| Get_Config_Data (a == 0) | `02 3A` | **`02 49`** | **+0x0F** | 2354 |
| **Feature GET** (report ID, 32 B) | | | | |
| Get_FW_Infos type 0 / 1 / 2 | `37` / `38` / `39` | **`47` / `48` / `49`** | +0x10 | 1781-1790 |
| Get_Serial_Number | `3A` | **`4A`** | +0x10 | 1748 `*puVar2 = 0x4a` |
| Get_Device_Infos | `3B` | **`4B`** (`'K'`) | +0x10 | 2198 `*pcVar1 = 'K'` |
| Get_Backlight | `3C` | **`4C`** | +0x10 | 1826 `*puVar1 = 0x4c` |
| Get_Animation_Infos | `3D` | **`4D`** (`'M'`) | +0x10 | 2593 `*pcVar1 = 'M'` |
| Get_Config_Infos | `3E` | **`4E`** | +0x10 | 1860 `*puVar1 = 0x4e` |
| **Input sub-type** (`rbuf[1]`) | `34..37` | **`44..47`** (`'D'..'G'`) | +0x10 | 2733 `thunk_FUN_1800072c0` |

> **Hazard.** If you apply +0x10 to the hardware-verified M4315 commands you get `03 4A 01` for "software mode".
> No M131E function uses `03 4A`, so the result is unknown. Worse, a +0x10 port of M4315 `03 3C`
> (Save_Factory_Data) lands on M131E **`03 4C` = Test_Factory_Reset**. Feature SETs on report `0x03` whose
> sub-command is `0x48`, `0x4B`, `0x4C` or `0x4D` are destructive. Never confuse them with the GET report
> IDs `0x48`/`0x4B`/`0x4C`/`0x4D`, which are harmless reads.

The HID descriptor `docs/descriptors/0206.bin` is byte-identical to `01fe.bin` except that the feature report
IDs are `0x47..0x4E` instead of `0x37..0x3E`. Output `0x02` carries 319 B, input `0x01` 63 B, feature `0x03` 31 B. (High)

---

## 1. Display

### 1.1 Display_Picture_On_Screen: Output `0x45`. Confidence: High (format), High (resolution)

`Display_Picture_On_Screen_CM` (`M131E.c:1879-2004`) is the M4315 function with only the command byte changed
(a diff against M4315's `@180006230` shows that `0x35`→`0x45` and the relocated symbols are the only differences):

```c
local_90 = param_1[10]; local_b0 = (u16*)(local_90 + 2);
*local_90 = 2;  local_90[1] = 0x45;
local_c4 = *(short*)(dev + 0xc) - 8;      // 320 - 8 = 312 data bytes per packet
*((u8*)local_b0 + 5) = 0;                 // buf[7] = last
do { if (rem <= 312) { buf[7] = 1; n = rem; }
     local_b0[0] = seq++;  local_b0[1] = n;     // buf[2..3], buf[4..5]
     memcpy(local_b0 + 3, img + (len - rem), n); // buf[8..]
     hid_write(dev, buf, 320); } while (rem -= n);
```

Packet: `02 45 seq:u16le len:u16le xx last data[≤312]`, zero-padded to 320 bytes.

- `buf[6]` is **never written** by this function, so it holds stale bytes. Send `0`. (High that the SDK
  doesn't set it; Low that the firmware ignores it.)
- No total length, CRC, ack, delay or commit. The packet with `last=1` triggers decode and display. (High)
- **Maximum size:** the SDK checks no length. The only hard limit is the u16 sequence number
  (65 536 × 312 B ≈ 20 MB). The real firmware buffer limit is unknown. The app sends canvas JPEGs at
  `quality: 1.0` (`knobs.js:1027`), so the firmware evidently accepts JPEGs of tens of KB. Keep frames
  ≲ 64 KB until this is measured. (Low)
- **Accepted formats:** the validator `FUN_180006c60` (`M131E.c:2503`) is identical to M4315's: `BM…`
  (24-bit BMP, normalised as in the M4315 doc) or a JPEG with `FF E0` (JFIF APP0) at offset 2. **Use baseline JFIF JPEG.** (High)
- **Resolution: 240 × 240.** (High) Evidence:
  - every renderer path draws or resizes to 240: `m131e.js:146` `image.resize(240, 240)`, `knobs.js:820/1116`
    `const size = 240`, `knobs.js:1105` `new Jimp(240, 240, '#000000')` (clear), and the GIF export uses `targetWidth: 240`
  - `Save_Screen_Picture` flashes in blocks of **0x38400 = 230 400 = 240 × 240 × 4** bytes (`M131E.c:2409`
    `if (0x383ff < local_38 + uVar2)`), against M4315's 0x7F800 = 480 × 272 × 4. So the block is one 32-bpp frame
    at panel resolution, which also confirms M4315's 480×272.
- **Shape:** probably a **round** panel. The UI preview uses `border-radius: 50%` (`DialEncoderPreview.vue:243-264`),
  text is clipped to 200 px wide, and 1.3″ 240×240 is the usual round GC9A01-class size. Content in the
  corners is likely invisible. (Medium)
- **Orientation:** the app applies no flip or rotate for M131E. (High for the app; whether "up" matches the
  physical mounting is a hardware question.)
- **GIF / animation:** host-streamed, one full `0x45` upload per frame. Main letter-boxes non-square GIFs onto a
  black 240² canvas (`background.pretty.js:48438-48460`). The app re-renders sensor screens every 1000 ms.

### 1.2 Display_Color_On_Screen: SET `[03 46 R G B]`. Confidence: High

`M131E.c:2006-2035`. Parameter order is (r, g, b). The addon **does not export** this for M131E (only
`Adjust_Backlight`, `Display_Picture_On_Screen`, `Set_Sw_Mode`, `Get_FW_Infos`, `Check_IO_Status` and `Update_FW`;
see `stackaddon.node.c:17551-17669`), so the app clears the screen by sending a black 240² JPEG instead. Testing this command is safe.

### 1.3 Backlight. Confidence: High

- Set: `[03 44 lvl]`, `lvl` 0..100 (`if (param_2 < 0x65)`, `M131E.c:1582`).
- Get: GET `0x4C`, `level = buf[2]` (`M131E.c:1826-1829`). The addon does not export the getter.

---

## 2. Input events

### 2.1 Framing. Confidence: High

`Check_Rcv_Data_CM` (`M131E.c:2041`) is identical to M4315's except for the sub-type table (`thunk_FUN_1800072c0`, `M131E.c:2733`):

| `type` param | `rbuf[1]` | Use |
|---|---|---|
| 0 | `0x44` `'D'` | **Dial events**: the only type the app polls (`stackaddon.node.c:13540` `Check_Rcv_Data_CM_M131E(dev, buf, timeout, 0)`) |
| 1 | `0x45` `'E'` | unknown (Low: config/animation reply) |
| 2 | `0x46` `'F'` | unknown, returned raw as 64 B (Low: config data stream) |
| 3 | `0x47` `'G'` | unknown, held back while type-2 reports are queued (Low: end-of-stream) |

Report: `01 44 len:u16le payload…`. The SDK hands the app `payload = rbuf[4 .. 4+len)` as `data[]`.

### 2.2 Dial payload. Confidence: Medium (byte meaning from JS), Low (len and data[1])

Main process (`background.pretty.js:48031-48045`):

```js
if (g = Kr.Check_IO_Status_CM_M131E(s, 0, sn), 0 === g) {
  const t = 4 === s[0] && 1 === s[2];            // press-down
  if (t) da[sn] = { pressedAt: new Date };       // swallowed, not forwarded
  else { zr("replyM131IoStatus", { data: s,
           longPress: da[sn] && Date.now() - da[sn].pressedAt >= 600, ... }); delete da[sn]; }
}
```

Renderer (`App.vue:7494-7590`):

```js
var signal = process.env.VUE_APP_DEVICE_TYPE != "CES" ? 4 : 1;
const isClick = (data[0] === signal && data[2] === 0) || (data[0] === 1 && data[2] === 0);
...
const isRight = data[2] === 0 && data[0] === 3 || data[2] === 0 && data[0] === 0
const isLeft  = data[2] === 1 && data[0] === 3 || data[2] === 1 && data[0] === 0
```

| `data[0]` (report byte 4) | `data[2]` (byte 6) | Meaning |
|---|---|---|
| `4` | `1` | **button pressed (down)** |
| `4` | `0` | **button released (up)** |
| `1` | `0` / `1` | release / press in an older ("CES") firmware. The release value is still accepted |
| `0` or `3` | `0` | **one detent clockwise ("right")** |
| `0` or `3` | `1` | **one detent counter-clockwise ("left")** |

- `data[1]` (report byte 5) is never read. By analogy with the hardware-verified M0 reports (fader
  `02 05 …`, roller `01 02 …`, where `data[1]` is the channel count), it is probably `01`. The expected
  report is then **`01 44 03 00 <kind> 01 <val>`**. (Low: verify the length and `data[1]`.)
- Why rotation uses two kind values (0 and 3) is unknown. Candidates: rotate while pressed versus normal, or
  two firmware revisions. (Low)
- **Steps and velocity:** the app treats each report as **exactly one step** and reads no count or speed
  field. If the firmware sends a step count for fast spins, it would have to be in `data[1]` or `data[3+]`,
  and the app ignores it. (Medium that there is one step per report.)
- **Long press** is a host-side construct. A press-down is stored, and the next report (normally the release) is
  forwarded with `longPress = held ≥ 600 ms`. So long press fires **on release**, not after 600 ms of holding.
  Rotating while held consumes the press timestamp, and the subsequent release then counts as a click.
  A Linux driver should time it properly: fire long press at 600 ms while held, and suppress the click on release.
- Return codes 7 (read error) → treated as unplug; 8 (no handle) → `Set_Sw_Mode(true)` + reconnect (`background.pretty.js:48047`).

---

## 3. Control and info

### 3.1 Set_Sw_Mode: SET **`[03 49 mode]`**. Confidence: High (byte), Medium (semantics)

`M131E.c:1543-1568`. `mode` = 1 software / 0 hardware, as on M4315. The app sends `true`:
- on every connect, unconditionally (`background.pretty.js:47605`)
- on resume (`46991`), on reconnect after rc 8, before FW update (`48140`), and on leaving the intro (`Initial.vue:102`)

It sends `false` on suspend (`46966`), on quit or settings sync (`47472`), and on disconnect (`47102`). M4315 hardware testing
showed that events, images and several feature GETs work only in software mode. Assume the same for M131E.

### 3.2 Device info: GET `0x4B`. Confidence: High (SDK copy), Low (meaning)

`M131E.c:2184-2220`. The SDK checks `buf[0] == 'K'`, then copies **less than the M4315 SDK does**:

```c
*param_2 = buf[2];
*(u16*)(param_2 + 2) = *(u16*)(buf + 1);
*(u16*)(param_2 + 4) = *(u16*)(buf + 3);
```

The global `DAT_180011e60` is written during connect and **never read**: unlike M4315, there are no index bound checks.
Expect the M4315 layout `[4b len type rows cols keys w:u16 h:u16 ? kw kh]`. For a dial that would plausibly be
`… 01 01 01 f0 00 f0 00 …` (240 = `f0 00`). This is a guess; dump it. (Low)

### 3.3 FW info: GET `0x47` / `0x48` / `0x49`. Confidence: High

Same layout as M4315: `[id len crc/ver:4 "vv.vv.vv.vv"(11)]` (`M131E.c:1770-1806`). `Get_FW_Infos_CM_M131E`
uses **type 1 → `0x48`** (`M131E.c:910`). From the M4315 hardware results, `0x47` is probably the bootloader version.

### 3.4 Serial number: GET `0x4A`. Confidence: High

`buf[1]` = length, `buf[2..]` = ASCII (`M131E.c:1748-1752`). The serial prefix is unknown (M4315 uses `MHKD15…`).

### 3.5 Config: GET `0x4E`, and Output `0x48`/`0x49`. Confidence: Medium

Same as M4315 §3.6 with the shifted bytes. The app does not use them.

---

## 4. DANGEROUS (do not implement; listed only so they can be recognised in captures)

| Function | Wire (M131E) | Notes |
|---|---|---|
| Update_FW_by_type | Output `[02 44 blk blkLast seq:u16 len:u16 final fwType … data≤304 @16]` | Identical to M4315 apart from the byte (`M131E.c:1608-1729`). FW < 0x50000. Embedded animation ≤ 0x176FFF B is flashed first via Save_Screen_Picture. `Update_FW_CM_M131E` = by_type(…, 1) (Ghidra mislabels it as `wcstoul` @180003480) |
| Save_Screen_Picture (animation/logo flash) | Output `[02 47 seq:u16 len:u16 slot first blockEnd last crc32:u32 data≤306 @14]` | Block 0x38400 B (one 240×240×4 frame). Polls GET `0x4D` status byte 0x10 after each block (`M131E.c:2378-2479`) |
| Set_Serial_Number | SET `[03 45 len≤29 ascii…]` | Overwrites the device identity |
| Test_FW_Recovery_Process | SET `[03 48]` | Probably reboots to the bootloader (PID `0x9994`) |
| Save_Factory_Data | SET `[03 4B]` | Commits factory data |
| Test_Factory_Reset_Process | SET `[03 4C]` | Factory reset |
| Test_Animation_Recovery_Process | SET `[03 4D]` | Restores the default animation |

---

## 5. Minimal safe driver sequence

1. Open the hidraw node for `2516:0206`. Ignore `0x9994` (bootloader).
2. SET `[03 49 01]` (32 B, zero-padded) to enter software mode.
3. GET `0x4B` (device info), `0x48` (FW version) and `0x4A` (serial). These are read-only; log the raw bytes.
4. SET `[03 44 <0..100>]` to set the backlight.
5. Upload a 240×240 baseline JFIF JPEG: `02 45 seqLE lenLE 00 last data[≤312]`, zero-padded to 320 B.
   Optionally test `[03 46 R G B]` first; it is the simplest visible command.
6. Read reports in a loop. Handle `r[1] == 0x44`: `r[4]==4` → button (`r[6]` 1 = down, 0 = up). `r[4]∈{0,3}` →
   rotation (`r[6]` 0 = CW, 1 = CCW, one step per report). Ignore other sub-types.
7. On exit, SET `[03 49 00]` to return to hardware mode (the built-in animation).

## 6. Open questions (verify on hardware)

1. Does `[03 49 01]` enter software mode? (High from the SDK, but untested.) Does the M131E respond in any way to `03 4A`? Don't test that.
2. The exact event report: length field, `data[1]`, which of `data[0]` 0 or 3 is used for rotation, and whether
   fast spins send one report per detent or a count. Detents per revolution.
3. Is "CW = 0" physically clockwise when viewed from the front?
4. Device-info (`0x4B`) layout: does it carry 240/240?
5. Firmware max JPEG size, progressive/EXIF JPEG handling, whether `buf[6]` matters, and the sustainable frame rate.
6. Is the panel round, and how is the 240² frame mapped onto it (centre, rotation)?
7. Which feature GETs return empty before software mode (on M4315 some do).
8. Meaning of input sub-types `0x45`–`0x47`.
