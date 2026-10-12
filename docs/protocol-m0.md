# M0 family wire protocol (Base, Fader, Knob, Roller)

Status: **static analysis only**. Nothing here has been checked against a USB capture yet.

Sources:
- `CoolTEK_SDK_CM_M0_V01.05.00.dll`, a Ghidra decompilation of the vendor SDK. It embeds the Windows hidapi.
- `stackaddon.node`, the N-API glue that calls the SDK. It shows the real argument values the app passes.
- MasterHUB app renderer source (`App.vue`, `lightingEffectMixin.js`, `masterHub.js`, `allModules.js`, `knobs.js`) and `background.js`. These show how the returned bytes are interpreted.

Each item is tagged with a confidence level:
- **[H]**: read directly from the decompiled code.
- **[M]**: inferred from how the app uses the data.
- **[L]**: a guess.

---

## 1. Devices

| Module  | VID:PID     | Alt PID tried by addon | SDK "LED group" | Check_Rcv_Data type |
|---------|-------------|------------------------|-----------------|---------------------|
| Base    | `2516:01f6` | `0x9999` (after prod fails) | 0 (index 0)     | 1 (`0x2B`)          |
| Fader   | `2516:0200` | `0x9991` (tried **first**, then `0x0200`) | 1 (index 0..4)  | 0 (`0x2A`)          |
| Knob    | `2516:0202` | `0x9996`               | 2 (index 0..2)  | 0 (`0x2A`)          |
| Roller  | `2516:0204` | `0x9995`               | no LED calls    | 0 (`0x2A`)          |

**[H]** The addon (`stackaddon.node`, connect wrappers at `FUN_1800070f0`/`FUN_180007340`/`FUN_180007580`, etc.) calls `Connect_Device_CM_M0(h, 0x2516, pid, serial)`. If the result is neither 0 nor 10, it retries with the alternate PID. The alternate PIDs are probably bootloader or pre-production IDs **[L]**.

Devices are opened by VID, PID and **serial number string** (at most 29 chars, otherwise error 6). Each module has its own handle.

## 2. Transport

Three kinds of transfer are used, all on the vendor page `0xFF00` collection.

| Direction | Mechanism | Report ID | Size on the wire (incl. ID) |
|-----------|-----------|-----------|-----------------------------|
| host to dev | Output report via `hid_write` (interrupt OUT / `WriteFile`) | `0x02` | 64 |
| dev to host | Input report via `hid_read_timeout` (interrupt IN) | `0x01` | 64 |
| dev to host | `hid_get_feature_report` (GET_REPORT control) | `0x2D`..`0x36` | 32 |

**[H]** The SDK **never** calls `hid_send_feature_report`. All writes are output report `0x02`, and all synchronous reads are GET_FEATURE. Feature report `0x03` is not referenced by the SDK at all.

### Output report `0x02` framing [H]

```
byte 0  : 0x02            report ID
byte 1  : command opcode  (0x2A..0x4A, see §3)
byte 2..: command args
```

- The SDK writes exactly `output_report_length` bytes (`*(ushort*)(dev+0xc)`, 64). Success means `hid_write` returned 64. Any other return value is error **3**.
- The SDK reuses one `calloc`'d output buffer (`dev+0x50`) and **never clears it between commands**. Bytes past the ones a command sets therefore hold stale data from earlier commands. The firmware evidently ignores them. **A Linux driver should zero-pad.**
- No checksum, no sequence number, and no ACK is read back for any `0x02` command, except that firmware update uses its own chunk counters (§3.9). Writes are fire-and-forget.

### Feature GET framing [H]

```
request : buf[0] = feature report ID, length 0x20 (32)
reply   : buf[0] = report ID, buf[1] = payload length N (usually), buf[2..2+N) = payload
```

- **Windows hidapi quirk:** this hidapi's `hid_get_feature_report` returns `bytes_returned + 1` (`iVar2 = local_38 + 1`). Every SDK getter therefore checks `ret == 0x21` (33), and anything else is error **3**. On Linux hidraw, `HIDIOCGFEATURE` returns **32** for the same transfer, so compare against 32.
- Only `Get_Device_Infos` (`0x31`) validates the echoed report ID (`buf[0] == '1'`, otherwise error 4). The other getters trust the reply.
- Most getters `memcpy(dst, buf+2, buf[1])` with **no bound check against the caller's buffer**. For example, the app gives `Get_Slider_Value` a 5-byte `Buffer`. Clamp `N` on your side.

## 3. Output commands (report `0x02`)

Common prelude for every exported call: `FUN_1800042b0(dev)` returns 8 if the handle is NULL, 1 if the output buffer is NULL, and otherwise continues.

### LED addressing: `group` / `index` [H, meaning M]

`Adjust_LED_Mode/Color/Brightness` all take `(int group, byte index, ...)`. The SDK validates them as follows:

| group | meaning (from addon) | valid index |
|-------|----------------------|-------------|
| 0     | Base                 | `0` only |
| 1     | Fader (5 LEDs, one per fader) | `0..4` or `0xFF` |
| 2     | Knob (3 LEDs, one per knob)  | `0..2` or `0xFF` |

```c
if (param_2 == 0)      { if (param_3 != 0) return 6; }
else if (param_2 == 1) { if ((4 < param_3) && (param_3 != 0xff)) return 6; }
else if (param_2 == 2) { if ((2 < param_3) && (param_3 != 0xff)) return 6; }
```

- The check is `param_2 < 3` on a **signed** int, so negative groups pass validation. Treat that as an SDK bug.
- `0xFF` presumably means "all LEDs in group" **[M]**. The app never sends it. The addon instead loops `index = 0..4` (Fader) or `0..2` (Knob) and sends one report per LED, returning only the first call's status.

### 3.1 `Adjust_LED_Mode_CM_M0(dev, group, index, mode, save)`: opcode `0x2C` [H]

| byte | value |
|------|-------|
| 0 | `0x02` |
| 1 | `0x2C` |
| 2 | group (0/1/2) |
| 3 | index |
| 4 | mode, `0..9` (SDK rejects anything else with 6) |
| 5 | save flag (byte, the 5th arg) |

Mode enum, from the app's `effectList` in `masterHub.js` **[M]**:

| mode | name | color used | brightness used |
|------|------|-----------|-----------------|
| 0 | disable / off | – | – |
| 1 | static | yes | yes |
| 2 | breathing | yes | yes |
| 3 | strobe | yes | yes |
| 4 | "spring" (hidden in the UI) | yes | yes |
| 5 | cycling (color cycle) | no | yes |
| 6 | random | no | yes |
| 7 | wave | no | no |
| 8 | water | no | yes |
| 9 | rainbow | no | yes |

The app always passes `save = false` for mode. Persisting presumably means storing the setting in device flash **[L]**.

### 3.2 `Adjust_LED_Color_CM_M0(dev, group, index, r, g, b, save)`: opcode `0x2D` [H]

| byte | value |
|------|-------|
| 1 | `0x2D` |
| 2 | group |
| 3 | index |
| 4 | R (0..255) |
| 5 | G |
| 6 | B |
| 7 | save flag |

The color order is **R, G, B [H]**. The addon passes the JS `{r,g,b,saveFw}` values straight through as args 4..7. The app sends `saveFw` (normally `false`).

### 3.3 `Adjust_LED_Brightness_CM_M0(dev, group, index, value)`: opcode `0x2E` [H]

| byte | value |
|------|-------|
| 1 | `0x2E` |
| 2 | group |
| 3 | index |
| 4 | brightness |

- The SDK does **no range check**. The UI slider is `min=1 max=100`, the default is 50 or 60, and `0` is never sent because the JS skips falsy values. So the range is **1..100 [M]**.
- This command has no save flag.

### 3.4 `Adjust_LED_Speed_CM_M0(dev, speed)`: opcode `0x30` [H]

| byte | value |
|------|-------|
| 1 | `0x30` |
| 2 | speed `0..100` (`param_2 < 0x65`, otherwise 6) |

- There is no group or index, so speed is device-global.
- The app does not use this command. The direction of the scale (fast vs slow) is unknown **[L]**.

### 3.5 `Calibrate_Direction_CM_M0(dev)`: opcode `0x2F` [H]

Sends `[0x02, 0x2F]` with no arguments. It probably recalibrates or re-detects the Base orientation (horizontal/vertical) **[M]**. Read the result with feature `0x35` or the input event in §4.4. The app does not call it.

### 3.6 `Get_Config_Data_CM_M0(dev, a, b)`: opcode `0x32` / `0x33` [H opcodes, L meaning]

- Exactly one of `a`, `b` must be nonzero. If both are nonzero, the call returns 6.
- `a == 0` sends `[0x02, 0x33]`. Otherwise (`a != 0`, `b == 0`) it sends `[0x02, 0x32]`.
- The call reads nothing back. It is probably a "please send config" trigger, with the answer arriving as input reports of sub-type `0x2D`/`0x2E` (Check_Rcv_Data types 3/4, §4) **[L]**.
- The app does not use this command.

### 3.7 DANGEROUS: `Set_Serial_Number_CM_M0(dev, buf, len)`: opcode `0x2B`

`[0x02, 0x2B, len (≤29), ascii[len]]`. This overwrites the USB serial, and the app opens devices **by serial**. **Never send it.**

### 3.8 DANGEROUS: factory / test commands

| Export | Bytes | Notes |
|--------|-------|-------|
| `Save_Factory_Data_CM_M0` | `02 31` | Commits current settings as factory data **[L]** |
| `Test_FW_Recovery_Process_CM_M0` | `02 49` | Probably forces bootloader / recovery mode **[L]** |
| `Test_Factory_Reset_Process_CM_M0` | `02 4A` | Factory reset |

None of these take arguments or have an ACK, and the app does not call any of them.

### 3.9 DANGEROUS: firmware update, `Update_FW_by_type_CM_M0(dev, data, size, type)`: opcode `0x2A`

`Update_FW_CM_M0(dev, data, size)` is the same call with `type = 1`. Ghidra mislabels it as `wcstoul @ 180003770`. The app uses it with `CM_M0_<Module>_V<ver>_download.sys`.

Size limits:
- `type == 0`: `size ≤ 0x0FFF`
- `type ≥ 1`: `size < 0xA000` (40 KiB)

The image is split into 4 KiB blocks. Each block is sent as 48-byte chunks (`out_len - 16`). Chunk layout:

| byte | value |
|------|-------|
| 1 | `0x2A` |
| 2 | block number (`offset / 0x1000`) |
| 3 | 1 on the last chunk of this block, else 0 |
| 4..5 | u16 LE chunk sequence within block (starts at 0) |
| 6..7 | u16 LE chunk data length (≤ 48) |
| 8 | 1 on the last chunk of the last block, else 0 |
| 9 | `type` |
| 10..15 | not set (stale buffer content) |
| 16..63 | data |

- If `size` is a multiple of 4096, the final "block" is sent as one zero-length chunk with the end flags set.
- There is no read-back, ACK, or checksum. A failed write prints `Err_TransferFailed` and returns 3.

## 4. Input reports (`0x01`) and `Check_Rcv_Data_CM_M0`

### 4.1 Wire format [H]

```
byte 0    : 0x01                 report ID (anything else -> error 4)
byte 1    : sub-type 0x2A..0x2E  ('*' '+' ',' '-' '.')
byte 2..3 : u16 LE payload length L
byte 4..  : payload (L bytes, ≤ 60)
```

`type = sub-type - 0x2A`:

| type (`param_4`) | sub-type | Delivered to caller | Used by |
|------------------|----------|---------------------|---------|
| 0 | `0x2A` | payload (`raw+4`, L bytes) | Knob, Fader, Roller events |
| 1 | `0x2B` | payload | Base events (orientation) |
| 2 | `0x2C` | payload | unused by app |
| 3 | `0x2D` | **whole raw 64-byte report** incl. ID | unused by app; probably config/data dump **[L]** |
| 4 | `0x2E` | payload, but only once no type-3 report is pending | unused by app; probably end/status of a type-3 stream **[L]** |

The `type` → sub-type mapping is from `FUN_1800051e0`:

```c
if (param_2 == 0) uVar1 = (param_1[1] == '*');   // 0x2A
else if (param_2 == 1) uVar1 = (param_1[1] == '+'); // 0x2B
...
else if (param_2 == 4) uVar1 = (param_1[1] == '.'); // 0x2E
```

### 4.2 `Check_Rcv_Data_CM_M0(dev, out, timeout_ms, type)` algorithm [H]

The SDK keeps **five 64-slot ring buffers**, one per sub-type. They are `dev+0x60` (slots), `+0x68` count[5], `+0x70` write_idx[5], and `+0x78` read_idx[5]. All counters wrap with `(x+1) & 0x3F`.

1. If `count[type] != 0`, dequeue from the ring (`FUN_180005490`) instead of reading the device:
   - copy the report to `out` (whole report for type 3; for the other types, `raw+4` for `raw[2..3]` bytes);
   - zero the slot, decrement the count, advance read_idx, and return **0**;
   - exception: if type 4 is requested while `count[3] != 0`, return **4** without dequeuing.
2. Otherwise call `hid_read_timeout(dev, buf, 64, timeout_ms)`:
   - `0` bytes: return **2** (nothing pending / timeout);
   - `< 0`: return **7** (read error, which the app treats as unplugged);
   - `buf[0] != 0x01`: return **4**.
3. If `buf[1]` matches `type`, copy the data as in step 1 and return **0**. There is one exception: when type 4 is requested while `count[3] != 0`, the report is queued instead and the call returns **6**.
4. If `buf[1]` is a different known sub-type (`FUN_180005380`), store it in that sub-type's ring and return **6**. If it is not `0x2A..0x2E`, return **4**.

So **6 from Check_Rcv_Data means "got an event for another type, queued it", not a bad argument**.

**Overflow bug:** if 64 reports queue up, `count` wraps to 0 and the ring silently loses them.

The app polls every module with `timeout = 0` (non-blocking) and a 256-byte `Buffer`. It handles return codes as follows:
- `0`: dispatch the event;
- `8`: reconnect;
- `7`: mark the module disconnected.

### 4.3 Event payloads (as seen by the app, `data[n] = raw[4+n]`) [M]

The interpretation below comes from `App.vue`. `data[0]`/`data[1]` are partly unknown.

**Knob (`0x0202`, sub-type `0x2A`)**, three knobs that the app labels B, R, G:

| condition | meaning |
|-----------|---------|
| `data[0] == 3` | **button press** event. `data[2]==1`: knob "B" pressed; `data[3]==1`: knob "R"; `data[4]==1`: knob "G" |
| otherwise | **rotation** event. `data[2]`, `data[3]`, `data[4]` belong to knobs B, R, G. `0` = counter-clockwise ("left") step, `1` = clockwise ("right") step, any other value (presumably `0xFF`) = no movement |

- Each report is one detent. The app adds or subtracts 5 per event, so no absolute position is reported.
- Other `data[0]` values for rotation are not checked by the app **[L: probably 1 or 2]**.

**Roller / Wheel (`0x0204`, sub-type `0x2A`)**, two wheels:

| byte | meaning |
|------|---------|
| `data[2]` | right wheel: `0xFF` = no event, `1` = "down" step, other (presumably `0`) = "up" step |
| `data[3]` | left wheel: same encoding |

These are relative step events with no magnitude. Roller buttons, if any, are not handled by the app.

**Fader / Slider (`0x0200`, sub-type `0x2A`)**:

| byte | meaning |
|------|---------|
| `data[2..6]` | absolute position of faders 0..4, **0..100** (the app maps `value/100` linearly to volume ranges) |

The device sends a report whenever any fader moves, and each report carries all five values.

**Base (`0x01f6`, sub-type `0x2B`)**:

| byte | meaning |
|------|---------|
| `data[0]` | orientation: `1` = horizontal, anything else = vertical |

The Base has no key events in this SDK.

## 5. Feature reports (GET only)

| ID | Export | Reply layout | Present on (descriptor) | Conf. |
|----|--------|--------------|-------------------------|-------|
| `0x2D` | `get_FW_Infos_by_type(…,0)` | `[2..5]` 4 bytes, `[6..16]` 11-char ASCII string | all M0 | H layout / L meaning |
| `0x2E` | `get_FW_Infos_by_type(…,1)`, `Get_FW_Infos` | same | all M0 | H |
| `0x2F` | `get_FW_Infos_by_type(…,2)` | same | all M0 | H |
| `0x30` | `Get_Serial_Number` | `[1]` = len, `[2..]` ASCII serial | all M0 | H |
| `0x31` | `Get_Device_Infos` | `[0]` must echo `0x31`; `[2]` = 1-byte device info/type, stored in a global and otherwise unused | all M0 | H layout / L meaning |
| `0x32` | `Get_LED_Brightness` | `[1]` = len, `[2..]` up to 5 bytes, presumably per-LED brightness | Base, Fader | H layout / M meaning |
| `0x33` | `Get_Slider_Value` | `[1]` = len (5), `[2..6]` fader 0..4 positions, 0..100 | Fader | H / M |
| `0x34` | `Get_LED_Mode` | `[1]` = len, `[2..]` up to 5 bytes, presumably per-LED mode | Base, Fader | H / M |
| `0x35` | `Get_Device_Direction` | `[2]` = orientation (`1` = horizontal, presumably) | Base | H / M |
| `0x36` | `Get_Config_Info` | `[1]` = len, `[2..]` up to 16 bytes, unknown | all M0 | H / L |

Firmware info details [H]:
- `Get_FW_Infos(dev, ver12, extra4)` reads `0x2E`. It returns `ver12` = bytes `[6..16]` + NUL (the app shows this as the FW version string, 11 chars) and `extra4` = bytes `[2..5]`.
- Types 0/1/2 (`0x2D`/`0x2E`/`0x2F`) are probably bootloader / application / hardware or data-region versions **[L]**.

Notes:
- The Knob descriptor was not dumped (`0x0202` is missing from `docs/descriptors/`), so which feature IDs it supports is unconfirmed.
- The Roller descriptor only has `2D–31` and `36`. That is consistent with it having no LEDs and no direction sensor.
- Base lacks `0x33` but has `0x35` (direction).
- Connect calls `Get_Device_Infos` (`0x31`) as a liveness check, so `0x31` must answer on every module.

## 6. Connect / disconnect [H]

`Connect_Device_CM_M0(&dev, vid, pid, serial)`:
1. `hid_init`. On failure, return 9.
2. If `serial` is longer than 29 chars, return 6. The serial is converted to wide chars.
3. If `dev` is already set, it reopens: success returns **10** (already connected, and the second handle is leaked), failure disconnects and returns **9**.
4. Otherwise call `hid_open(vid, pid, serial)`. If that returns NULL, return 9.
5. It allocates an output buffer (`out_len`), an input buffer (`in_len`), and 64×5 ring slots. On allocation failure it returns 1.
6. It calls `Get_Device_Infos` (feature `0x31`) and returns that status: 0 means OK.

Windows `hid_open_path` also calls `HidD_SetNumInputBuffers(…, 0x40)`, so 64 input reports are buffered by the OS.

`Connect_Device_byPath_CM_M0` works the same way, but opens by path. `Disconnect_Device_CM_M0` frees the buffers and closes the handle; if it was not connected, it returns 8.

## 7. Return / error codes [H]

| code | meaning |
|------|---------|
| 0 | success |
| 1 | device struct not initialised (output buffer NULL) or allocation failure |
| 2 | `Check_Rcv_Data`: no input report within timeout |
| 3 | I/O failure: `hid_write` did not write a full report, or the feature GET length was not 33 (Windows) |
| 4 | unexpected data: input report ID ≠ `0x01`, unknown input sub-type, feature `0x31` ID mismatch, or type 4 blocked by pending type 3 |
| 6 | invalid argument (NULL buffer, range check). **From `Check_Rcv_Data`, this means a report of another sub-type was received and queued** |
| 7 | `hid_read_timeout` error, meaning the device is gone |
| 8 | handle is NULL (not connected) |
| 9 | `hid_init` failed or the device could not be opened |
| 10 | already connected (`Connect_*` called on an open handle) |

## 8. Minimal Linux implementation notes

- Use hidraw:
  - writes: `write(fd, [0x02, op, args…] zero-padded to 64)`;
  - reads: `read(fd, buf, 64)`, where `buf[0] == 0x01`;
  - feature: `HIDIOCGFEATURE(32)` with `buf[0] = id`, expecting a return of 32.
- Example: set every Fader LED to static red at 100% brightness:
  ```
  02 2C 01 FF 01 00   (or index 00..04 individually, like the app does)
  02 2D 01 FF FF 00 00 00
  02 2E 01 FF 64
  ```
  `0xFF` "all" is untested. Send per index if it does not work.
- Event decode: see §4.3. Ignore sub-types you don't care about. There is no need for the SDK's ring buffers when one reader consumes all reports.

## 9. Open questions

1. Meaning of input sub-types `0x2C`, `0x2D` and `0x2E`, and whether `Get_Config_Data` (`0x32`/`0x33`) triggers them.
2. Meaning of `data[0]`/`data[1]` in `0x2A` events. Only knob `data[0]==3` (press) is known.
3. Whether the roller wheels have a push/click.
4. Whether LED index `0xFF` really addresses all LEDs, and what the save flag persists.
5. LED speed scale direction, and whether speed applies per mode.
6. The 4-byte field in FW info, the meaning of FW-info types 0/2, the `0x31` device-info byte, and the `0x36` config blob.
7. The Knob (`0x0202`) report descriptor, which still needs to be dumped.
8. The Base orientation value set for `0x35`, and what `Calibrate_Direction` actually does.
