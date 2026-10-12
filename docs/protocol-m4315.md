# CM_M4315 "15xKey w/ IPS" module: wire protocol

Device: VID `0x2516`, PID `0x01fe` (USB high-speed). The add-on also tries PID **`0x9997`** when
`0x01fe` fails to open, which is probably the bootloader / recovery PID.
Product string: `15xKey w/ IPS`, serial prefix `MHKD15`. The app calls it "15 Keys Control Pad".

Sources (static analysis only; nothing here comes from a capture yet):

- `decomp/CoolTEK_SDK_CM_M4315_V02.00.00.dll.c`: vendor SDK with embedded Windows hidapi (primary source)
- `decomp/CoolTEK_SDK_CM_M131E_V02.00.01.dll.c`: sibling SDK. It uses the same code with every command byte shifted by +0x10
- `decomp/stackaddon.node.c`: N-API add-on that calls the SDK
- `background.pretty.js` and `hub_src/src/**`: the MasterCTRL Electron app

Confidence levels: **High** means it is read directly from the decompiled code. **Medium** means the
code is clear but the semantics are inferred. **Low** means it is a guess.

---

## 0. Transport conventions

The HID report descriptor (`docs/descriptors/01fe.bin`) defines these reports:

| Report | Dir | Payload | Used for |
|---|---|---|---|
| `0x02` | Output (interrupt OUT) | 319 B | bulk data: images, FW, animation, config requests |
| `0x01` | Input (interrupt IN)   | 63 B  | events and replies |
| `0x03` | Feature (SET)          | 31 B  | all "set" commands, with the sub-command in byte 1 |
| `0x37`..`0x3E` | Feature (GET)  | 31 B  | each report ID is a "get" query |

SDK conventions (High):

- **Output writes are always the full 320 bytes** (report ID + 319). The SDK always calls
  `hid_write(dev, buf, *(u16*)(dev+0xc))`, where `dev+0xc` = OutputReportByteLength = 320.
  The SDK reuses the same 320-byte buffer (`dev+0x50`) for every command **and never clears it**,
  so unused trailing bytes hold stale data from earlier packets. A Linux driver should zero-fill them.
- **Feature SET** is always `HidD_SetFeature(buf, 0x20)`: 32 bytes, `[0x03, subcmd, args..., 0-pad]`.
  A non-zero return means success.
- **Feature GET** is `HidD_GetFeature(buf, 0x20)` with `buf[0] = <report id>`. The SDK checks
  `ret == 0x21`, which is its hidapi's "bytes+1" convention, so 32 bytes were transferred. On Linux,
  use `HIDIOCGFEATURE(32)` with `buf[0]=id`. In the reply, `buf[0]` is the report ID, `buf[1]` is
  usually the payload length, and the data starts at `buf[2]`.
- On Linux hidraw: write `[0x02, ...319 bytes]`, read gives `[0x01, ...63 bytes]`,
  and `ioctl(HIDIOCSFEATURE(32), [0x03, sub, ...])`.

SDK return codes, useful for mapping errors: 0 OK, 1 out of memory or not open, 2 read timeout,
3 I/O error or short write, 4 unexpected data, 5 device-reported failure, 6 bad parameter (also
"stashed a report of another type", see section 2), 7 read error, 8 null handle, 9 open failed,
10 already open, 11 FW too large.

### Command byte map (High)

| Byte 1 value | via `0x03` Feature SET | via `0x02` Output | via Feature GET (report ID) |
|---|---|---|---|
| `0x34` | Adjust_Backlight | **Update_FW** (DANGEROUS) | (input event type 0, see section 2) |
| `0x35` | Set_Serial_Number (DANGEROUS) | **Display_Picture_On_Screen** | |
| `0x36` | Display_Color_On_Screen | **Display_Picture_On_Button** | |
| `0x37` | Display_Color_On_Button | | Get_FW_Infos type 0 |
| `0x38` | | Save_Screen_Picture / animation flash (DANGEROUS) | Get_FW_Infos type 1 |
| `0x39` | Test_FW_Recovery (DANGEROUS) | Get_Config_Data (variant b) | Get_FW_Infos type 2 |
| `0x3A` | **Set_Sw_Mode** | Get_Config_Data (variant a) | Get_Serial_Number |
| `0x3B` | | | **Get_Device_Infos** |
| `0x3C` | Save_Factory_Data (DANGEROUS) | | **Get_Backlight** |
| `0x3D` | Test_Factory_Reset (DANGEROUS) | | Get_Animation_Infos |
| `0x3E` | Test_Animation_Recovery (DANGEROUS) | | Get_Config_Infos |

The M131E SDK uses identical code with every value +0x10 (`0x45` = screen picture, `0x44` = backlight,
`0x4A` = sw mode, `0x4B` = device info, and so on). That suggests the M4315 values come from a
per-model base of `0x34` and are not arbitrary.

---

## 1. Image upload

### 1.1 Per-button picture: `Display_Picture_On_Button` (Output `0x02`, cmd `0x36`). Confidence: High

Evidence, from `Display_Picture_On_Button_CM @ 180005bb0`:

```c
local_88 = param_1[10];               // 320-byte out buffer
local_b0 = (u16*)(local_88 + 2);
*local_88 = 2;  local_88[1] = 0x36;
...
local_c4 = *(short*)(dev + 0xc) - 8;  // max chunk = 320 - 8 = 312
*((u8*)local_b0 + 5) = 0;             // buf[7] = last flag
*(u8*)(local_b0 + 2) = param_2;       // buf[6] = button index
do {
  if (remaining <= 312) { buf[7] = 1; chunk = remaining; }
  local_b0[0] = seq++;                // buf[2..3]
  local_b0[1] = chunk;                // buf[4..5]
  memcpy(local_b0 + 3, data + (total - remaining), chunk);  // buf[8..]
  remaining -= chunk;
  hid_write(dev, buf, 320);
} while (remaining != 0);
```

Packet layout (320 bytes on the wire, including the report ID):

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x02` report ID |
| 1 | 1 | `0x36` command: picture on button |
| 2 | 2 | packet sequence number, **u16 LE**, starts at 0 for each image and increments per packet |
| 4 | 2 | number of valid data bytes in this packet, **u16 LE**, at most **312** |
| 6 | 1 | button index, 0-based |
| 7 | 1 | `1` on the final packet of this image, otherwise `0` |
| 8 | n | image data bytes `[offset .. offset+n)`, where offset = sum of previous chunk lengths |
| 8+n | 312-n | padding. The SDK leaves stale bytes here; send zeros |

- There is **no explicit byte offset** in the packet. The firmware reassembles by sequence number and
  concatenation, and the first packet is the one with `seq=0`.
- There is **no total-length field** and **no CRC** in this command. End-of-image is signalled only by `buf[7]=1`.
- If the image fits in one packet (≤312 B), that single packet has seq 0 and last=1.
- Packets per image = ceil(len / 312). For example, a 72×72 JPEG of about 3 KB takes about 10 packets.
- **There is no commit or flush command.** The final packet (last=1) makes the device decode and display the image.
- **There is no ack or delay.** The SDK writes the packets back-to-back. Each `hid_write` is an overlapped
  WriteFile that waits for completion, with a timeout constant `_DAT_18000fa50` that the same code
  uses as a ms scale, so probably 1000 ms. The SDK reads nothing after an image. Interrupt OUT is
  HS with bInterval 1 (125 µs) and a 320-byte max packet, so one report is one USB transaction.
- Bounds check: `button <= DeviceInfo.byte[3] - 1` (section 3.5), so for this device indexes 0..14.

#### Accepted payload formats (validator `FUN_180007440`). Confidence: High

```c
if (p[0]=='B' && p[1]=='M') ok;                 // Windows BMP
else if (p[2]==0xFF && p[3]==0xE0) ok;          // JPEG with JFIF APP0 directly after SOI
else return 6;
```

1. **JPEG (what the app actually sends).** The bytes are passed through **unchanged**. The SDK only checks
   that `FF E0` (JFIF APP0) is at offset 2, so a JPEG whose first marker is EXIF (`FF E1`) is
   rejected by the SDK. The firmware may not care, but to be safe emit baseline JFIF.
2. **BMP, 24-bit (3 bytes/px, BGR).** The SDK does some host-side normalisation before sending:
   - It keeps the original 54-byte (`0x36`) header **unchanged**.
   - If `width % 4 != 0`, it strips the per-row padding (`thunk_FUN_1800044e0` copies `width*3` bytes
     per row) and sets length = `54 + w*h*3`.
   - If `biHeight > 0` (bottom-up), it reverses the row order to top-down (`thunk_FUN_1800043a0(...,3)`),
     **but the header still holds the positive height**. The firmware therefore expects top-down
     rows whatever the header says (Medium).
   - It sends `header(54) + w*h*3` bytes of BGR. With a width that is already a multiple of 4, the
     caller's length is used as-is.
   - The pixel-size constant `DAT_18000fa7c` is not in the dump, but it must be 3.0 given the `width*3` copies.

   For a driver, **JPEG is the recommended format**. BMP is a fallback, and nothing shows that the
   device understands RGB565.

#### Image dimensions and preparation in the app. Confidence: High for 72×72, Medium for orientation

- Every button image the app produces is **72×72 JPEG**, made with Jimp `getBufferAsync(MIME_JPEG)`,
  which always writes a JFIF APP0:
  - `keypad.js:3394`: `smallBuffer = await img.resize(72, 72).getBufferAsync(Jimp.MIME_JPEG);`
  - `keypad.js:2103/2169` (reset or clear a key): `new Jimp(72, 72, '#000000')` encoded as JPEG and sent to `Display_Picture_On_Button_CM_M4315`
  - `15keys.vue:516`: `imagePrev.resize(72, 72); imagePrev.quality(60);`
  - Press feedback (`App.vue funcPlusCount`): the original image is shrunk to 36×36 and composited
    onto a 72×72 black canvas at (20,20), then sent as JPEG
- **No flip or rotate** is applied for M4315 (`flip(true,true)` is commented out in every M4315 path;
  only M4300 flips). The SDK contains rotate helpers (`FUN_180003ae0` for an arbitrary angle and
  `FUN_180003fa0` for a 90° transpose) but **nothing calls them**.
- GIF buttons: the app decodes the GIF into per-frame 72×72 JPEG buffers (`keypadBuffers.js`, `frames[]`,
  `framesDelay`) and **streams frames itself** with a ~30 ms scheduler (`universalGifInterval`), one
  full `0x36` upload per frame. The device has no per-button animation upload.

#### Button index mapping. Confidence: Medium-High

- `btnId` is the 0-based index into the app's `keypadBtnList[...][page]`. The UI grid is
  `grid-template-columns: repeat(5, 1fr)` (15keys.vue:857), so **5 columns × 3 rows, row-major, 0 = top-left**
  in the module's "vertical" orientation. In "horizontal" orientation the app only rotates the on-screen
  preview (`rotate(270deg)`); the index sent to the device is the same.
- The **same index** comes back in key events (section 2): the key with `payload[2+k]=1` is
  `btnId k`, and the app re-sends the image to `btnId: func.id` for that same k.
- M4315 uses no index remap. Compare M3704, which uses `3 - btnId`.

### 1.2 Full-screen picture: `Display_Picture_On_Screen` (Output `0x02`, cmd `0x35`). Confidence: High for format, Low for dimensions

The code is identical to 1.1 except:

```c
*local_90 = 2;  local_90[1] = 0x35;
// buf[6] is NOT written; it holds whatever the previous command left (button idx etc.)
```

The layout is the same: `[02][35][seq u16][len u16][buf6 = don't care, send 0][last][data ≤312]`.
JPEG and BMP are accepted and handled the same way.

- **The MasterCTRL app never calls this for M4315.** The IPC handler exists in `background.pretty.js:47808`, but nothing in `hub_src` sends it.
- **The full-screen resolution is unknown.** The keys appear to be windows onto a single IPS panel,
  which would put it around 5×72 by 3×72 plus bezels, but that is unverified. `Get_Device_Infos` bytes
  8..11 (two u16) are the best candidate for panel width and height (section 3.5). Read them first.

### 1.3 Solid colour fills (Feature SET `0x03`). Confidence: High

```c
// Display_Color_On_Button_CM
buf = {0x03, 0x37, idx, R, G, B, 0...}   // 32 bytes
// check: idx <= DevInfo[1]*DevInfo[2]   (off-by-one allows idx == 15)
// Display_Color_On_Screen_CM
buf = {0x03, 0x36, R, G, B, 0...}
```

Parameter order in the SDK signature is (idx, r, g, b), and the app passes `t.r, t.g, t.b`.
The app clears an empty key with `Display_Color_On_Button (idx, 0,0,0)`.

---

## 2. Input events (Input report `0x01`, 64 bytes incl. ID)

### 2.1 Report framing. Confidence: High

`Check_Rcv_Data_CM(dev, out, timeout_ms, type)`:

```c
n = hid_read_timeout(dev, rbuf, 64, timeout);
if (n == 0) return 2;               // timeout
if (rbuf[0] != 0x01) return 4;
if (rbuf[1] == "4567"[type]) {      // type 0..3 -> 0x34..0x37
   if (type == 2) memcpy(out, rbuf, 64);                         // whole report
   else           memcpy(out, rbuf + 4, *(u16*)(rbuf + 2));      // payload
   return 0;
} else stash rbuf in ring[type_of(rbuf[1])]; return 6;
```

Input report layout:

| Offset | Field |
|---|---|
| 0 | `0x01` report ID |
| 1 | message type: `0x34`, `0x35`, `0x36` or `0x37` (= `Check_Rcv_Data` type 0..3) |
| 2..3 | payload length, u16 LE (at most 60) |
| 4.. | payload |

**Meaning of the `type` parameter:**

| type | `rbuf[1]` | Meaning | Confidence |
|---|---|---|---|
| 0 | `0x34` | **Key state / IO status.** This is the only type the app polls (`Check_IO_Status_CM_M4315 -> Check_Rcv_Data_CM_M4315(dev, buf, 0, 0)`) | High |
| 1 | `0x35` | unknown; probably a reply to a config or animation request | Low |
| 2 | `0x36` | unknown, returned raw (all 64 bytes); probably config data chunks after `Get_Config_Data` | Low |
| 3 | `0x37` | unknown. It is held back while type-2 reports are queued (`if type==3 && count[2]!=0` the SDK stashes it or returns 4), so it is likely an end-of-transfer marker for a type-2 stream | Low |

SDK queueing: for each type there is a 64-slot ring (`dev+0x68`, with count/wr/rd in `dev+0x70/0x78/0x80`).
A report of another type gets queued and the call returns 6. The next call for that type is served
from the ring first. A Linux implementation can simply demultiplex on `rbuf[1]`.

### 2.2 Key event payload (type 0, `0x34`). Confidence: Medium-High

From `App.vue checkKeyRelease()`:

```js
var statusIndex = arrBuf.indexOf(1) - 2;
//arrbuf format :[0, 15, 0, 0, 0, ...]
this.keyPressList.forEach((v, k) => { if (v === 1 && arrBuf[k + 2] === 0) { /* release of key k */ } });
if (statusIndex > -1) this.funcPlusCount(statusIndex, ...);   // press of key statusIndex
```

`arrBuf` is the payload (report bytes 4..). Inferred layout:

| Report offset | Payload offset | Value |
|---|---|---|
| 4 | 0 | `0x00` (unknown, possibly page or bank) |
| 5 | 1 | `0x0F` = 15 (key count) |
| 6+k | 2+k | state of key k (0..14): `1` = pressed, `0` = released |

So the full report is probably `01 34 11 00 00 0F s0 s1 … s14`, with length 0x11 = 17 (length not confirmed).
This is a **full state snapshot** on every change, not an edge event. The app compares it with its own
`keyPressList` to derive press and release. Multi-key handling in the app is naive (`indexOf(1)` sees
only the first pressed key). A driver should diff all 15 bytes.

Polling: the app calls Check_IO_Status with **timeout 0** (non-blocking) in a loop. The SDK sets
`HidD_SetNumInputBuffers(0x40)`. Events only arrive in **software mode** (`Set_Sw_Mode 1`,
section 3.2). A return of 7 is treated as unplugged and 8 as reconnect.

---

## 3. Control and info (Feature reports)

### 3.1 Backlight. Confidence: High

- Set: `[0x03, 0x34, level]` with `level` 0..100 (`if (param_2 < 0x65)`, else error 6).
- Get: GET feature `0x3C`, `level = buf[2]`.
- The app sends one global "brightness" slider value to all M131E and M4315 modules
  (`adjust_brightness`). This is the **LCD backlight**; there is no separate RGB key light.

### 3.2 Set_Sw_Mode. Confidence: High for encoding, Medium for semantics

- `[0x03, 0x3A, mode]`
- The app only ever passes a JS boolean, so mode is **`1` = software (host-driven) mode** or
  **`0` = hardware (standalone) mode**:
  - On connect: `Set_Sw_Mode_CM_M4315(true, sn)` and the app takes over the keys and LCDs.
  - On system suspend or app exit: `Set_Sw_Mode_CM_M4315(!1, e)`, logged as `"→ hardware mode"`.
    The device then presumably shows its built-in or flashed content (logo or animation).
  - While the app is "initialising" (`Jr` flag), it sends false.
- No other values are seen. Do not probe other values blindly.

### 3.3 Firmware info: GET feature `0x37` / `0x38` / `0x39`. Confidence: High for layout

`Get_FW_Infos_by_type(type)`: type 0 reads `0x37`, type 1 reads `0x38`, type 2 reads `0x39`.

| Reply byte | Field |
|---|---|
| 0 | report ID |
| 1 | (length?) |
| 2..5 | 4 bytes (unknown: numeric version, build or checksum) |
| 6..16 | 11 bytes ASCII version string (the add-on returns these 11 bytes to JS) |

`Get_FW_Infos_CM_M4315` uses **type 1** (likely the main app FW). Types 0 and 2 are likely the
bootloader and the animation/resource partition (Low). The app compares the result with
`CM_M4315_V<version>_download.sys`.

### 3.4 Serial number: GET feature `0x3A`. Confidence: High

Reply `buf[1]` = length, `buf[2..]` = ASCII serial. This is read-only and safe. Setting it is DANGEROUS (section 4).

### 3.5 Device info: GET feature `0x3B`. Confidence: High for layout, Low for meaning

The SDK calls this on every connect and checks `buf[0] == 0x3B`. The SDK copies the reply into a
global used for bounds checks:

| Reply byte | Global | Use / guess |
|---|---|---|
| 2 | `info[0]` | ? (device type) |
| 3 | `info[1]` | multiplied by `info[2]` gives the colour-fill index bound, so probably columns (5) |
| 4 | `info[2]` | probably rows (3) |
| 5 | `info[3]` | **button count** (picture bound: `idx <= info[3]-1`), expected 15 |
| 6, 7 | `info[4]`, `info[5]` | ? |
| 8..9 | u16 | ? possibly panel width or key image width |
| 10..11 | u16 | ? possibly panel height or key image height |

### 3.6 Config: GET `0x3E`, and Output `0x39`/`0x3A`. Confidence: Medium

- `Get_Config_Infos`: GET feature `0x3E`. Reply `buf[1]` = length, `buf[2..]` = up to 16 bytes, meaning unknown.
- `Get_Config_Data(a, b)`: sends a 320-byte Output `[0x02, 0x3A]` if `a==0`, or `[0x02, 0x39]` if `a!=0 && b==0`.
  The rest of the buffer is stale and there are no parameters. The answer presumably arrives as
  input reports of type 1, 2 or 3. The app does not use this function.

### 3.7 Animation info: GET `0x3D`. Read-only, but it belongs to the DANGEROUS flash path

`buf[0]` must equal `0x3D`, `buf[1]` = length, and the payload is copied to a global:
- payload bytes 0..3: u32 LE ID or CRC of stored animation slot "type 1"
- payload bytes 8..11: u32 LE for slot "type 0"
- payload byte 0x10: **flash state machine status**: 0 = idle (retry up to 6× at 10 ms),
  1 = busy (poll every 10 ms), 2 = busy (poll every 1 s, probably erasing), 3 = error, 4 = busy (100 ms),
  5 = ready for the next block

---

## 4. DANGEROUS commands (do not implement in a normal driver)

All of these write persistent device state or the firmware. They are documented only so they can be recognised in captures.

| Function | Wire | Notes |
|---|---|---|
| `Update_FW_by_type` | Output `[02][34][blk][blkLast][seq u16][len u16][final][fwType][pad..15][data ≤304 @16]` | File is split into 4 KiB blocks (`blk` = block number; `seq` restarts per block). Image length = `*(u32*)(file+0x1c) + 0x24`. FW must be < 0x50000, or < 0xFFFF when fwType=0. If an embedded animation (≤0x177000 B) differs by ID from `Get_Animation_Infos`, it is flashed first with `Save_Screen_Picture`. No acks during the FW write. **Bricking risk.** The bootloader PID is probably `0x9997`. |
| `Save_Screen_Picture` (animation / boot screen to flash) | Output `[02][38][seq u16][len u16][slot][first][blockEnd][last][CRC32 LE u32 @10][data ≤306 @14]` | Standard CRC-32 (poly 0xEDB88320) over each packet's data. Blocks are 0x7F800 B, after which `seq` resets. After every short (block-end or last) packet, the host polls GET `0x3D` status (see 3.7). Writes flash. |
| `Set_Serial_Number` | SET `[03][35][len ≤29][ascii...]` | Overwrites the serial number, which is also the app's device identity. |
| `Save_Factory_Data` | SET `[03][3C]` | Commits factory data. |
| `Test_Factory_Reset_Process` | SET `[03][3D]` | Factory reset. |
| `Test_FW_Recovery_Process` | SET `[03][39]` | Probably reboots into recovery/bootloader. |
| `Test_Animation_Recovery_Process` | SET `[03][3E]` | Probably restores the default animation. |

Note that the "replace logo" and "display logo" IPCs (`Replace_Logo_CM_M4315`, `Display_Logo_CM_M4315`) are referenced by
`keypad.js` but are **not exported by this SDK version**. They probably correspond to `Save_Screen_Picture`.

---

## 5. Minimal safe driver sequence (proposed)

1. Open the hidraw node for 2516:01fe.
2. GET `0x3B` (device info) and check that `buf[0]==0x3B`. Optionally GET `0x38` (FW version) and `0x3A` (serial).
3. SET `[03 3A 01]` to enter software mode.
4. SET `[03 34 <0..100>]` to set brightness.
5. For each key k in 0..14: send 72×72 baseline JFIF JPEG chunks: `02 36 seqLE lenLE k last data[≤312] zero-pad` → 320 B.
6. Read input reports in a loop. When `r[1]==0x34`, diff `r[6..20]` against the previous state to get press and release events.
7. On exit, SET `[03 3A 00]` to return to hardware mode.

## 6. Open questions (verify with usbmon)

1. Full-screen (`0x35`) resolution and whether the firmware cares about `buf[6]`. Meaning of the device-info u16s at reply bytes 8..11.
2. Whether 72×72 is the native per-key resolution or the firmware scales. Behaviour with BMP and with non-JFIF JPEG (progressive, EXIF).
3. The exact type-0 payload: the length field and whether payload[0] has a meaning (page or layer?).
4. Meaning of input types `0x35`/`0x36`/`0x37` and of the `Get_Config_Data` replies.
5. Whether the device needs inter-packet pacing when several keys are updated back-to-back at GIF frame rates (the SDK uses none).
6. Physical orientation: whether key 0 is top-left when the module is mounted "vertical" or "horizontal".
