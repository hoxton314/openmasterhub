# MasterHUB vendor app: application-level architecture

Status: derived from static analysis of the vendor Windows app (Electron main
`background.pretty.js`, the Vue renderer sources recovered from source maps
under `hub_src/src`, the `stackaddon.node` native addon, and the
`CoolTEK_SDK_CM_*` DLL decompiles). This file covers **what the app does**.
Wire formats are documented separately in `PROTOCOL.md`.

Conventions used below:

- "main" = Electron main process (`background.pretty.js`; minified names are given as they appear).
- "renderer" = Vue app (`hub_src/src/...`).
- "SDK buffer" = the 256-byte `Buffer` that main passes to `Check_IO_Status_CM_<model>(buf, 0, serial)`. The byte indices quoted are the ones the renderer reads. How they map onto raw HID input reports belongs in `PROTOCOL.md`. The M0 SDK's `Check_Rcv_Data_CM_M0` copies the payload that follows a 4-byte report header.

---

## 1. Product table

VID is always `0x2516` (Cooler Master). Each module has a production PID and a
`0x99xx` engineering/alt PID. `startBaseDeviceCheck` (main) accepts **both**.
The addon's `connect*` functions try one PID and then fall back to the other.

| Internal model | App names (UI / FW server) | Prod PID | Alt PID | SDK family / addon prefix | Inputs | Outputs | Footprint (grid cells) |
|---|---|---|---|---|---|---|---|
| **M0x64**, Base | "BaseBoard", `basex64`, `MODEL_BASE='m0x64'` | `0x01f6` | `0x9999` | `CM_M0` (`connectM0_Base`, `*_CM_M0_Base`) | Orientation event only (vertical/horizontal) | ARGB LEDs: mode/color/brightness | 4 wide × 6 tall |
| **M4315**, 15-key IPS pad | "15 Keys Control Pad", `15keys`, `'m4315'` | `0x01fe` | `0x9997` | `CM_M4315` (separate DLL) | 15 keys (press/release) | 15 × **72×72** per-key LCD images (JPEG), LCD backlight 0–100, `Set_Sw_Mode` | 4 × 3 |
| **M005S**, 5-fader | "Slider", `sliders5`, `'m005s'` | `0x0200` | `0x9991` | `CM_M0` (`connectM0_Slider`) | 5 faders, absolute 0..100 | ARGB LEDs | 4 × 2 |
| **M003**, 3-knob | "3 x Knobs Module", `knob3`, `'m003'` | `0x0202` | `0x9996` | `CM_M0` (`connectM0_Knob`) | 3 endless encoders (`rotateR/G/B`) + push on each | ARGB LEDs | 2 × 2 |
| **M002**, 2-roller | "Scrolling Wheel", `roller`, `'m002'` | `0x0204` | `0x9995` | `CM_M0` (`connectM0_Wheel`) | 2 rollers (`left`, `right`), detent steps | none used | 4 × 1 |
| **M131E**, 1.3" dial | "1.3” Dial Encoder", `dialEncoder`/`knobx1`, `'m131'` | `0x0206` | `0x9994` | `CM_M131E` (separate DLL) | 1 encoder + push (click / long-press) | **240×240** round LCD (JPEG), backlight 0–100, `Set_Sw_Mode` | 2 × 2 |
| M3704, 4-key (unreleased) | `control-4key`, reported as `'m4315'` | – | `0x9989` | `CM_M3704` | 4 keys | 4 × **152×152** JPEG images (main rotates them 270° and uses `btnId' = 3 - btnId`), backlight | 4 × 1 |
| M3700 (unreleased) | – | – | `0x9988` | `CM_M3700` | ? | screen | not in the JS |
| M4300 (unreleased) | `small-m4300` | – | `0x9998` | `CM_M4300` | – | **480×272** screen, logo replace | detected only, with no serial number |
| M4600 (unreleased) | "touch46", `display-m4600` | – | `0x9993` | `CM_M4600` | touch: tap at x/y (16-bit LE) | **800×320** screen | 4 × 2 |
| M5000 (unreleased) | "touch50", `display-m5000` | – | `0x9992` | `CM_M4600` (main calls `connectM4600` for it, a vendor bug) | touch: tap at x/y | **800×480** screen | 4 × 3 |
| Prolific hub | `deviceBase` | `067b:2586` | – | none; used only for topology | – | – | – |

Sources: `startBaseDeviceCheck` handler (main, around line 47646, decimal PIDs
`512/39313`, `518/39316`, `516/39317`, `514/39318`, `510/39319`,
`502/39321`, `39305`, `39320`, `39314`, `39315`, VID `9494`, hub `1659:9606`);
`stackaddon.node.c` around lines 4374–5109 (`Connect_Device_CM_*` with PIDs);
`types/constant.js`; `store/module/masterHub.js:3075-3094` (screen sizes);
`TopNav.vue` (firmware-server module names).

Notes:

- The shipping product is Base + M4315 + M005S + M003 + M002 + M131E. The rest are prototypes that still have code paths.
- Exported addon functions (`stackaddon.node.c:17515-17567`): `initCpuid fetchSysInfo getAppsAudioVolume setApplicationVolume enableSkipWinKey disableSkipWinKey initIOHookCallback startIOHook stopIOHook connect*/disconnect* (M4315 M3700 M3704 M0_Knob M0_Wheel M0_Slider M0_Base M131 M4300 M4600 M5000 21) Get_FW_Infos_* Update_FW_* Check_IO_Status_* Get_Slider_Value_CM_M0 Adjust_LED_{Mode,Brightness,Color}_CM_M0_{Base,Knob,Slider} Check_IO_Status_All_M0 Adjust_Backlight_* Display_Picture_On_Screen_* Replace_Logo_CM_M4300 Display_Logo_CM_M4300 Display_{Color,Picture}_On_Button_CM_{M4315,M3704} Set_Sw_Mode_CM_{M131E,M4315} ...`.
- **`Get_Device_Direction_CM_M0` and `Calibrate_Direction_CM_M0` exist in the M0 DLL but the addon does not export them, and the app never calls them.** For what they do, see section 2.4.
- Every device is addressed by its **USB serial-number string**, which `Ea()` reads in main via node-usb `getStringDescriptor(iSerialNumber)`. Every addon call takes the serial as its last argument. Each module also has the same serial as its key in the profile JSON.
- Firmware versions come from `Get_FW_Infos_CM_*`. Updates are `*_download.sys` files fetched from a CM firmware server and written with `Update_FW_CM_*` (main, around line 48050). During an update the serial goes into the lock map `k` (`N()`/`M()`), and the global `O=false` blocks every image write.

---

## 2. Topology and layout detection

### 2.1 Enumeration loop (`startBaseDeviceCheck`, main)

- The renderer starts it once (`App.vue initialApp()`, 2 s after mount). It then re-arms itself with `setTimeout(e, 2000)` for as long as the app runs (`Zr`).
- Each pass calls `usb.getDeviceList()` (node-usb/libusb). For every device:
  - `rootAddress E`: `xa(dev, dev.portNumbers.length)` walks `parent` up `len-1` times. That reaches the ancestor directly below the root hub (depth 1). Then `E = ancestor.deviceAddress + ancestor.busNumber`. This is an arithmetic **sum**, not a tuple, so it is fragile. It is `-1` when there are no `portNumbers`.
  - If VID:PID matches a module (either PID), main opens the device, reads its serial string, and pushes `{serialNumber, portNumbers, rootAddress}` into the per-model list.
  - Every `067b:2586` gets pushed to `deviceBase` as `{...dev, rootAddress}`.
- It sends `replyBaseDeviceCheck {deviceM131, deviceM005S, deviceM002, deviceM003, deviceM4315, deviceM3704, deviceM4300, deviceM4600, deviceM5000, deviceBase, deviceM0x64}`.
- If `VUE_APP_FORCE_SHOW_DEVICE` is set, main sends fabricated device lists instead (debug).

On Linux, `portNumbers` is the sysfs path (`<bus>-<p1>.<p2>.<p3>…` gives `[p1,p2,p3…]`). Use `(busnum, p1)` instead of the address sum.

### 2.2 Parent and port resolution (`App.vue arrayContainsSerialNumber(deviceBase, basex64, moduleRow)`, line 572)

For each module row, the renderer runs this:

```
parentSerialNumber = ""; portLength = 0
for hub in deviceBase (Prolific hubs, libusb order):
    if hub.portNumbers is not a prefix of module.portNumbers: continue
    # (vendor also compares a.rootAddress !== b.rootAddress, but a/b are the
    #  portNumbers arrays, so the test is undefined===undefined and has no effect)
    base = first M0x64 row such that
             base.rootAddress == module.rootAddress
         and len(base.portNumbers) - len(hub.portNumbers) >= 2
         and hub.portNumbers is a prefix of base.portNumbers
    if base: parentSerialNumber = base.serial; portLength = len(hub.portNumbers)
          # the last hub that matches wins
module.parentSerialNumber = parentSerialNumber
module.portNumber = "".join(str(p) for p in module.portNumbers[portLength:])
```

In practice, the only hub that satisfies "Base MCU is at least 2 levels below the hub" is the **top-level Prolific hub inside the Base**. `portNumber` therefore becomes the path below that top hub as a digit string. A commented-out sample in `App.vue` around line 7930 confirms this:

```
M003  portNumbers [10,1,1,3]  ->  top hub at [10]  ->  portNumber "113"
Base  portNumbers [10,1,4,4]  (Base MCU = top-hub p1 -> hub p4 -> port 4, i.e. "144")
```

Topology inferred from the strings the UI understands (27 module connectors, matching the CSS comment "pos 27"):

```
top hub A (in Base)
 ├─ A.p1 → hub B
 │        ├─ B.p1 → hub → ports 1..4   => "111" "112" "113" "114"
 │        ├─ B.p2 → hub → ports 1..4   => "121" .. "124"
 │        ├─ B.p3 → hub → ports 1..4   => "131" .. "134"
 │        └─ B.p4 → hub → ports 1..3   => "141" "142" "143"   (p4 = "144" = Base MCU 0x1f6)
 ├─ A.p2 → hub → ports 1..4            => "21" .. "24"
 ├─ A.p3 → hub → ports 1..4            => "31" .. "34"
 └─ A.p4 → hub → ports 1..4            => "41" .. "44"
```

Modules whose `parentSerialNumber` is `""` (not under a Base) are **not shown**. Every view filters with the `filterParent*Alive` getters (`store/module/allModules.js`), keyed by `currentBase64SerialNumber`. Several Bases can coexist, and the UI shows one at a time.

The Base's own `portNumber` is the full path. A vendor bug (`slice({})`) makes this happen, and nothing uses the value.

### 2.3 Port string to grid slot (`Baseboard.vue` `*Pos()` and CSS `.<module>.pos<portNumber>`)

The renderer turns `portNumber` straight into a CSS class `pos<portNumber>`. The
Base canvas (`.baseBlock`) is 556×814 px with 25 px padding. Cells are 129 px,
so the grid is **4 columns × 6 rows**, with (col 0, row 0) at top-left when the
Base is "vertical". The table below comes from the CSS `top/left/transform`. It
takes the module footprints from the widget CSS (knob 248² = 2×2,
keypad 507×379 = 4×3, slider 506×250 = 4×2, roller 506×~121 = 4×1) and rotates
them about their centre. Read it as "**the module's USB connector lands on base
connector X, so the module occupies these cells with this rotation**". Ranges
are inclusive. "rot" is the module's rotation relative to its natural
orientation.

**M4315 15-key (`.control-keypad`, 4×3):**

| port | cols | rows | rot |
|---|---|---|---|
| 24 | 0-3 | 0-2 | 0 |
| 42 | 0-3 | 1-3 | 0 |
| 41 | 0-3 | 2-4 | 0 |
| 122 | 0-3 | 3-5 | 0 |
| 141 | 0-3 | 0-2 | 180 |
| 33 | 0-3 | 2-4 | 180 |
| 132 | 0-2 | 0-3 | 270 |
| 134 | 1-3 | 0-3 | 270 |
| 112 | 1-3 | 1-4 | 270 |
| 22 | 0-2 | 2-5 | 270 |
| 23 | 1-3 | 2-5 | 270 |

**M005S slider (`.control-sliders5`, 4×2):**

| port | cols | rows | rot |
|---|---|---|---|
| 113 | 0-3 | 0-1 | 0 |
| 24 | 0-3 | 1-2 | 0 |
| 42 | 0-3 | 2-3 | 0 |
| 41 | 0-3 | 3-4 | 0 |
| 122 | 0-3 | 4-5 | 0 |
| 141 | 0-3 | 0-1 | 180 |
| 33 | 0-3 | 2-3 | 180 |
| 143 | 0-1 | 0-3 | 270 |
| 132 | 1-2 | 0-3 | 270 |
| 134 | 2-3 | 0-3 | 270 |
| 32 | 2-3 | 0-3 | 270 |
| 112 | 2-3 | 1-4 | 270 |
| 111 | 0-1 | 2-5 | 270 |
| 22 | 1-2 | 2-5 | 270 |
| 23 | 2-3 | 2-5 | 270 |
| 31 | 2-3 | 2-5 | 270 |

**M003 3-knob (`.knob3`) and M131E dial (`.knobx1`), both 2×2 with identical maps:**

| port | cols | rows | rot | | port | cols | rows | rot |
|---|---|---|---|---|---|---|---|---|
| 24 | 2-3 | 1-2 | 0 | | 143 | 0-1 | 0-1 | 270 |
| 31 | 0-1 | 2-3 | 0 | | 132 | 1-2 | 0-1 | 270 |
| 42 | 2-3 | 2-3 | 0 | | 134 | 2-3 | 0-1 | 270 |
| 41 | 2-3 | 3-4 | 0 | | 112 | 2-3 | 1-2 | 270 |
| 123 | 0-1 | 4-5 | 0 | | 111 | 0-1 | 2-3 | 270 |
| 122 | 2-3 | 4-5 | 0 | | 22 | 1-2 | 2-3 | 270 |
| 121 (dial only) | 0-1 | 2-3 | 0 | | 23 | 2-3 | 2-3 | 270 |
| 141 | 0-1 | 0-1 | 180 | | 43 | 2-3 | 3-4 | 270 |
| 142 | 1-2 | 0-1 | 180 | | 124 | 0-1 | 4-5 | 270 |
| 131 | 2-3 | 0-1 | 180 | | 44 | 2-3 | 4-5 | 270 |
| 33 | 0-1 | 2-3 | 180 | | 113, 34 | col only in CSS (incomplete) | | |
| 32 | 1-2 | 2-3 | 180 | | 21 | 2-3 | 2-3 | 180 |

**M002 roller (`.roller`, 4×1), same classes as `.control-4key` (M3704):**

| port | cols | rows | rot | | port | cols | rows | rot |
|---|---|---|---|---|---|---|---|---|
| 134 | 0-3 | 0 | 0 | | 141 | 0 | 0-3 | 270 |
| 112 | 0-3 | 1 | 0 | | 142 | 1 | 0-3 | 270 |
| 23 | 0-3 | 2 | 0 | | 124, 131 | 2 | 0-3 | 270 |
| 43 | 0-3 | 3 | 0 | | 133 | 3 | 0-3 | 270 |
| 44 | 0-3 | 4 | 0 | | 33 | 0 | 2-5 | 270 |
| 121 | 0-3 | 5 | 0 | | 32 | 1 | 2-5 | 270 |
| 31 | 1 | 0-3 | 90 | | 21 | 2 | 2-5 | 270 |
| 42 | 3 | 0-3 | 90 | | 114 | 3 | 2-5 | 270 |
| 123 | 1 | 2-5 | 90 | | 41 | 3 | ~1-4 | 90 (odd offset in CSS) |
| 122 | 3 | 2-5 | 90 | | 13, 14, 34 | off-grid placeholder | | |

Caveats:

- These ranges are computed from pixel offsets (±a few px) and may be off by one where the vendor CSS is sloppy. Treat them as a starting point and confirm on hardware: plug a module into each spot and log `portNumbers`.
- The same port string can map to different cells for different module types, because each module's USB connector sits at a different place in its footprint. The lookup key must be **(module type, port string)**, not the port string alone.
- The vendor app assumes that (type, port) uniquely determines the rotation. It never asks the module for its orientation.

### 2.4 Base orientation (vertical/horizontal)

- The app does **not** call `Get_Device_Direction_CM_M0` (feature report `0x35`, byte 2) or `Calibrate_Direction_CM_M0` (output `02 2F`). Both live only in `CoolTEK_SDK_CM_M0_V01.05.00.dll.c:1165,1855`.
- Instead, `startCheckIoStatus` polls `Check_IO_Status_CM_M0_Base` for each Base. When `result==0`, the renderer (`App.vue` `replyM00x64IoStatus`, line 7419) commits `setM00x64Direction(serial, data[0]===1 ? 'horizontal' : 'vertical')`. The default is `vertical`. The Base therefore pushes an event whenever it is rotated. It probably has a tilt sensor, which would explain the existence of `Calibrate_Direction`.
- Orientation is purely cosmetic in the app. `Baseboard.vue` rotates the whole `.baseBlock` 90°, and some widgets counter-rotate labels and menus (`Direction.vue`, `15keys.vue`, `setCoordinate.js`). **Key images, encoder directions and slot mapping do not change.** `Direction.vue` also lets the user override it locally for 5 s, but sends nothing to the device.

---

## 3. Runtime loop

### 3.1 Start-up sequence

1. `app.whenReady` → `_a()` creates the BrowserWindow (900×500, frameless, tray, single instance, `masterhub://` deep link, `.mhz` file association → `confirmImportProfile`). It starts an express server on `:4040` for OAuth callbacks and connects socket.io to the sysinfo daemon on `:3636`.
2. The renderer's `App.vue mounted` loads `hubConfig.json` and `profiles.json`. 2 s later `initialApp()` sends `startBaseDeviceCheck`.
3. Each `replyBaseDeviceCheck` connects newly seen modules (`App.vue` line 7768) in this order: M4600, M5000, M131 (`connectM131`), M002, M003, M4315 (`sendSync connectM4315`, then 500 ms sleep), M3704, M005S, M0x64. The counter `delayFwCount` increments for each module and staggers `Get_FW_Infos` by 200 ms × n. New modules get default config. A new Base triggers profile load (`checkAllBasex64Modules` → `importConfig`). Hot-plugged ARGB modules get their LED state re-sent.
4. `views/Initial.vue` shows an intro animation. While it runs, main's `Jr` (initialising) is true and `connectM4315` puts keypads into **hardware mode**. When the user clicks through (or the app was auto-started with `--openAsHidden`), `goDashboard()` sets `flagStopInitalAnimation`, sends `Set_Sw_Mode_CM_M131E(true)` and `Set_Sw_Mode_CM_M4315(true)` for every module, and draws the dial LCD.
5. The `flagStopInitalAnimation` watcher sends `initialDone` (so `Jr=false`), starts the **IO poll loop** (`startCheckIoStatus`) and connects MasterCTRL after 3 s. It then loads the profile list, restores OBS/Streamlabs connections, pushes all ARGB lighting (`AllLightingHandler`) and starts the GIF scheduler (`universalGifInterval`).

### 3.2 Input polling (`startCheckIoStatus`, main around line 47880)

- One loop runs on the **Electron main thread**. It is async and re-arms itself with `setTimeout(e, 20)`, so it ticks about every 20 ms plus the time the calls take. **No native thread or callback delivers module input.**
- Each tick copies the alive lists (`sa`=M4315, `pa`=M3704, `la`=M4600, `ca`=M5000, `ia`=Base, `oa`=M131E, `na`=M002, `ra`=M003, `aa`=M005S). For each serial that is not being firmware-updated, it calls `Check_IO_Status_CM_<model>(Buffer.alloc(256), 0 /*timeout ms*/, serial)`. That is a non-blocking `hid_read_timeout(..., 0)` in the SDK, so **at most one report per module per tick** is read.
- Return codes (from `Check_Rcv_Data_CM_M0` and the app's handling):

| rc | meaning | app action |
|---|---|---|
| 0 | report received | `zr("reply<Model>IoStatus", {result, data: buf, serialNumber})` to the renderer |
| 2 | no data (timeout) | ignore |
| 4 | unexpected report ID | ignore |
| 7 | read error (unplugged) | `D({status:false})` → `disconnect*` + `deviceNotFoundHandler`, remove from the alive list |
| 8 | handle missing | reconnect (`connect*`). For M4315/M131E also `Set_Sw_Mode(true)` when not initialising |

- M131E long-press is detected **in main**. A report with `buf[0]==4 && buf[2]==1` (press) is swallowed and its time stored in `da[serial]`. The next report is forwarded with `longPress = (now - pressedAt) >= 600 ms`.
- M4600/M5000 touch is debounced in main too: `[flag, ?, xL, xH, yL, yH]`. When `flag==0`, main stores x/y. When `flag!=0`, it emits once with the stored x/y (a tap).
- Lost reports: polling at about 50 Hz with only one read per tick means fast roller or encoder spins queue up in the OS buffer. A Linux implementation should block on, or drain, each hidraw fd instead (one thread or epoll per device).

### 3.3 What `stackaddon` threads and callbacks actually are

- `initIOHookCallback(cb)`, `startIOHook()`, `stopIOHook()` (`stackaddon.node.c:4140-4320`, `FUN_1800068a0`): a native thread installs `SetWindowsHookExA(WH_KEYBOARD_LL=13)` and pumps messages with `Sleep(20)`. For `WM_KEYDOWN`/`WM_SYSKEYDOWN` and `WM_KEYUP`/`WM_SYSKEYUP` it calls the JS callback through `napi_call_threadsafe_function` with `(vkCode, GetKeyNameTextA name, "keydown"|"keyup")`. Main forwards these as `ioHook` and `ioHookKeyup` IPC events (with `Num n` renamed to `Numpad n`).
  **This hooks the PC keyboard, not the MasterHUB.** The UI uses it only to *record* hotkeys and macros (`RecordMacroModal.vue`, `HotKey.vue`, `KnobsDefault.vue`, `DialEncoderDefault.vue`, `RollerDefault.vue`). The Linux equivalent is capturing key events in the GUI widget; no global hook is needed.
- `enableSkipWinKey`/`disableSkipWinKey`: they swallow the Win key during recording.
- `initCpuid(cb)` + `fetchSysInfo()` (threadsafe fn `sysInfoTsfn`) are used only by `hub_daemon/main.js`; see section 5.6.
- `Check_IO_Status_All_M0` (`FUN_180008dc0`) is a leftover debug loop that opens 0x202/0x204 and printf's. The app does not use it.

### 3.4 Event semantics per module (renderer handlers in `App.vue`)

| Module | Handler | Decoding (SDK buffer) | Semantics |
|---|---|---|---|
| Base | `replyM00x64IoStatus` | `data[0]==1` horizontal, else vertical | orientation change |
| M4315 | `replyM4315IoStatus` → `checkKeyRelease(buf)` → `funcPlusCount` | `data[2+k]` = state of key k (0..14). Press: `indexOf(1)-2`. Release: key previously 1 (in `keyPressList`) now 0 | Edge-triggered press **and** release. Only the first pressed key per report is handled as a press. |
| M3704 | `replyM3704IoStatus` → same `checkKeyRelease` | same | – |
| M005S | `replyM005SIoStatus` | `data[2..6]` = fader 0..4, **absolute 0..100** (the comment says "row.value 0 ~ 100"). The initial value is read with `Get_Slider_Value_CM_M0(Buffer(5))` at connect. | Every report carries all 5 values. `updateSliderTogglingIndex` diff-tracks them and only the **first changed** fader (`isToggling`) drives volume/OBS/Streamlabs. `brightness` fires on every report. |
| M003 | `replyM003IoStatus` | `data[0]==3` → push event: `data[2]==1` knob B, `data[3]==1` knob R, `data[4]==1` knob G. Otherwise rotation: `data[2]`→B, `data[3]`→R, `data[4]`→G, each **0 = left/CCW, 1 = right/CW**, any other value (e.g. 0xFF) = no movement | One step per report. Push cycles that knob's `functionList` (`switchKnob3Function`). Rotation runs `executeKnob3Action(row.left | row.right)`. The UI also nudges a ±5 "RGB value" per step (`setRgbR/G/B`). |
| M002 | `replyM002IoStatus` | `data[2]` right wheel, `data[3]` left wheel. `255` = none, `1` = **down**, anything else (0) = **up** | One detent step per report → `executeRollerAction(roller.up | roller.down)` |
| M131E | `replyM131IoStatus` | click: `(data[0]==4 or 1) && data[2]==0`. Rotation: `data[0]∈{0,3}`, `data[2]==0` → **right/CW**, `data[2]==1` → **left/CCW** | Short click cycles `functionList`, draws the function card on the LCD and returns to the normal display after 2 s. Long press (≥600 ms) runs `executeDialEncoderPressAction` after 300 ms. Rotation runs `executeDialEncodeAction(row.right | row.left)`. |
| M4600/M5000 | `replyM4600IoStatus`/`replyM5000IoStatus` | x1/y1 tap coords | touch zones (unreleased) |

Key press visual feedback (M4315, `funcPlusCount` line 1185): on press, a 72×72 black JPEG is drawn with the button icon scaled to 36×36 at (20,20) (multiply blend, opacity 0.5/0.9). On release (`checkKeyRelease`) the original image is re-sent.

---

## 4. Output usage

### 4.1 Images (JPEG end to end; no RGB565 in JS)

- All images are built in the renderer, either with Jimp or by drawing on an HTML canvas and calling `toDataURL('image/jpeg', 1.0)`. They go to main as **JPEG buffers**, and main passes them **unchanged** to the SDK.
- M3704 is the one exception: main re-encodes its images with Jimp (`rotate(270).resize(152,152).quality(60)`).
- The SDK's `Display_Picture_On_Button_CM` (`CoolTEK_SDK_CM_M4315_V02.00.00.dll.c:1906`) checks for a `BM` header. A BMP gets its rows padded to 4 bytes and flipped bottom-up. Anything else, including JPEG, is chunked into output report 2, command `0x36`, with the button id. **The module firmware decodes the JPEG.** No RGB565 conversion happens anywhere on the host.

| Target | Call | Size | When |
|---|---|---|---|
| M4315 key | `Display_Picture_On_Button_CM_M4315(btnId 0..14, jpeg, len, sn)` | 72×72 (authoring canvas 200×200 → `smallBuffer` via `resize(72,72)`) | Page or folder render (`keypadSwitchRender`, `triggerSwitchRenderCount`), settings edits (`syncFuncIcon`), key press/release feedback, GIF frames, timer countdown (1 s), state icons (mute, play) |
| M4315 key (blank) | `Display_Color_On_Button_CM_M4315(btnId, 0,0,0, sn)` | – | empty button |
| M131E LCD | `Display_Picture_On_Screen_CM_M131E(jpeg, len, sn)` | 240×240 | Function card on click (2 s); `lcdDisplayMode` = `Default` (icon + name), `Image` (static or GIF), or one of `CPU Temperature`/`GPU Temperature`/`CPU Usage`/`GPU Usage` (**re-rendered every 1000 ms**, `startDialEncoderDisplayLCD`); timer countdown; black 240×240 to clear |
| M4300/M4600/M5000 | `Display_Picture_On_Screen_CM_*` | 480×272 / 800×320 / 800×480 | unreleased |

- Button text and labels are drawn into the image on the canvas: font `Rubik` 700 by default, sizes from `fontSizeByStep` `{1:20,2:28,3:36,4:44,5:16,6:32,7:64}`, plus colour and alignment.
- Images are pushed only for the **current page/folder**. Switching page or folder re-pushes all 15 keys. Nothing is cached on the device across page switches.

**GIFs** (main `resizeAndExportFramesGif`):

- Main copies the GIF and runs `gifsicle --loopcount=0`, then `--info` to read per-frame delays (default 100 ms).
- It resizes with `gifsicle --resize WxH -e -U`, which explodes the GIF into frames. Every frame becomes a JPEG.
  - M4315 (72×72): frames are stored in `gifPool/<md5-of-gif>` as JSON `{handleGifFrames:[Buffer...], copyFilePath, framesDelay:[ms...], gifFileHash}`.
  - M131E (240×240): aspect is preserved and the frame is letter-boxed onto a black 240² canvas.
- Playback is software-timed (`App.vue universalGifInterval`). The scheduler ticks every **30 ms** when any GIF is visible, otherwise every 1 s. A frame advances once accumulated time ≥ `max(frameDelay, 66 ms)` **and** ≥ 100 ms (`gifStableDuration`), and frames are skipped to stay in time. That caps the effective rate at about **10 fps per button/LCD**. Each frame is a full JPEG write.

### 4.2 Backlight and LEDs

- **LCD backlight** (M4315 and M131E, global): IPC `adjust_brightness(v)` calls `Adjust_Backlight_CM_M131E(v, sn)` and `Adjust_Backlight_CM_M4315(v, sn)` for **all** connected LCD modules. `v` is 0..100, and `allModuleBrightness` defaults to 50. Bound actions: the `brightness` key/knob/roller action (±5 per step) and the `brightness` slider (absolute 0..100).
- **ARGB LEDs** (Base, M003, M005S only; the M002 roller has none in the app):
  - Mode: `Adjust_LED_Mode_CM_M0_<X>(0, effect, false, sn)`.
  - Colour: `Adjust_LED_Color_CM_M0_<X>(r, g, b, saveFw=false, sn)`.
  - Brightness: `Adjust_LED_Brightness_CM_M0_<X>(0, 0..100, sn)`.
  - Effect ids: `0` off, `1` static, `2` breathing, `3` strobe, (`4` spring, hidden), `5` cycling, `6` random, `7` wave, `8` water, `9` rainbow. `10` and `11` are the "next" and "previous" pseudo-actions.
  - State lives in `deviceLightingEffect`: `{currentLightingEffectDevice:'single'|'whole', wholeDevice:{effect:'9', color:'#743ADF', brightness:50}, basex64:{sn:{…}}, m003:{sn:{…}}, slider:{sn:{…}}}`. It is pushed by `AllLightingHandler` at start-up, on hot-plug, and **every 60 s** (`syncLightingEffect`) to resync.
  - `saveFw` is always false, so the app never persists LED state to flash.

### 4.3 `Set_Sw_Mode`

`Set_Sw_Mode_CM(handle, bool)` sends feature report `[0x03, 0x3A, bool]` (32 bytes; `CoolTEK_SDK_CM_M4315...c:1568`). Only the two LCD modules (M4315, M131E) have it. When the app sets it:

| Event | Mode |
|---|---|
| `connectM4315` while the app is initialising (`Jr`) | hardware (false) |
| `connectM4315` after init, IO-loop reconnect (rc 8) | **software (true)** |
| `connectM131` (always), M131E firmware update | software |
| `Initial.vue goDashboard()` (user leaves the intro) | software, for all M4315 and M131E |
| `powerMonitor 'suspend'` | hardware |
| `powerMonitor 'resume'` (+2 s) | software, then `RECONNECT_*` → full re-render, then FW info staggered 200 ms each |
| Tray "Exit"; `fs.watch` on `%APPDATA%/MasterHub/systemSetting` fires (installer/updater) | hardware (after `SYNC_CONFIG` to save the profile) |

The IPC handlers `Set_Sw_Mode_CM_*` in main only act when the flag is truthy, so the renderer can only switch *to* software mode. Main itself switches back to hardware mode.

---

## 5. Action system and profile format

### 5.1 Binding model

- **Keys** (M4315 / M3704): `keypadBtnList[sn][page][0..14]`. Each entry is a button object; `defaultKeypadObj` is in `store/module/keypad.js:21`. Fields:
  - action: `btnFunc` (action id), `dropdownOption` (sub-option index), `hotkey {key:[names], code:[...]}`, `hotKeySettingMode 'hotkey'|'macro'`, `hotkeyMacro {keys:[{code, name, action:'Press'|'Release', seconds /*actually ms, ≤9999*/}]}`, `macro` (legacy "a,ctrl+b" string)
  - action parameters: `url`, `appFilePath`, `actionAppMode 'Open'|close`, `content` + `afterEnter` (text), `countDownSecondDuration` (timer), `volumeAction`, `volumeControlProcessId`, `profileId`, OBS/Streamlabs/Discord/Twitch fields (`sourceId`, `selectSceneId`, `visibilityScene`, `MuteMode`, `streamlabsAudioSourceId`, `discordSelectUser`, `slowChatDuration` …), `actionList` (multi-tasking)
  - folder: `folder:[15 buttons]`, where folder slot 0 is "back"
  - appearance: `displayType 'color'|'image'`, `btnColor`, `btnIconColor`, `btnTextColor`, `btnFontStyle`, `btnFontSizeStep`, `btnAlignText`, `enableTitle`, `title`, `iconPath`, `imageBuffer` (72² JPEG), `bigImageBuffer` (200²), `originalImageBuffer`, `imageType 'gif'|…`, `gifPath`, `gifFileHash`, `frames`, `smallFrames`
- Per-keypad UI state lives in `keypadBtnConfig[sn] = {currentPage, totalPage, pageList, viewMode 'master'|'folder', currentFolderIndex}`. Pages are unlimited (`create-new-page`, `next-page`, `previous-page`). `new-folder` enters folder view, and in folder view key 0 returns.
- **Encoders** (M003 `knobsList[sn].rotateR|rotateG|rotateB`, M131E `dialEncoder[sn]`, M002 `rollerBlock[sn].left|right`): `{btnFunc, btnName, functionIndex, functionList:[{btnFunc, btnName, left/right or up/down}], left|up:{dropdownOption, hotkey, volumeAction}, right|down:{…}, volumeRange:[-96,26] (OBS dB), streamlabsVolumeRange:[0,1], volumeControlProcessId, sourceId}`.
  - A knob or dial holds a **list of functions**, and pressing it cycles through them. Each direction runs a "step" of the action, usually `dropdownOption 0` = up/next/+ and `1` = down/prev/−.
  - The dial adds `lcdDisplayMode`, `bgColorHex`, `textColorHex`, image/GIF fields and a long-press action.
- **Faders** (M005S `sliders5List[sn][0..4]`): `{btnFunc, btnName, value 0..100, sourceId, streamlabsAudioSourceId, volumeControlProcessId, slider:{volumeRange, streamlabsVolumeRange, …}}`.

### 5.2 Action catalogue (`store/module/masterHub.js featureClassList`, executors in `App.vue`)

Supported modules are given as K = 15-key, M = usable inside multi-tasking, R = roller, S = slider, D = dial, N = 3-knob.

| Group | Action ids | Modules | Implementation (Windows) |
|---|---|---|---|
| MasterHUB | `multi-tasking` | K | runs `actionList` sequentially, 280 ms apart; each hotkey is released afterwards (`executeMutitaskingAction`) |
| | `lighting-effect` | K M | ARGB effect (§4.2) |
| | `profile` | K M D N | next/prev or a specific `profileId` → `selectProfile` |
| | `brightness` | K M S D N R | LCD backlight ±5 or absolute |
| | `timer` | K D | countdown drawn on the key or LCD every 1 s; optional sound |
| | `mh-zoom-in-out` | K M R D N | zooms the app UI (Ctrl+= / Ctrl+-) |
| Navigation | `new-folder` `create-new-page` `previous-page` `next-page` | K | keypad paging |
| System | `hot-key` (Key Actions: chord **held while the key is held**, or a recorded macro with Press/Release + delays), `undo-redo`, `switch-desktop`, `media-track-control`, `media-fast-back-forward`, `zoom-in-out`, `screen-capture` | K M R D N | synthetic keyboard via nut-js `keyboard.pressKey/releaseKey(...Key enum codes)`. Defaults per action in `store/helpers/setDefaultHotkey.js` and `setDefaultAdobeHotkey.js`. Codes are nut-js `Key` enum values (Windows VK map in `store/helpers/setVirtualCodeNutCode.js`). Encoders do press+release per step. |
| | `scroll-up-down` | K M R D N | `mouse.scrollUp/Down(100)` |
| | `volume-control` | K M S R D N | per-app or system volume. `get_audio_output_app.exe` lists sessions (`{processId, volume, mute}`). `nircmd.exe setappvolume /<pid> v/100`, `setsysvolume v*65535/100`, `muteappvolume`/`mutesysvolume`, debounced 200 ms. Steps are ±5. `processId 0` = system. |
| | `play-pause` ("Play Audio") | K M | soundboard: plays an audio file in the renderer `<audio>` |
| | `web-url` | K M | `explorer "https://…"` |
| | `app-files` | K M | launch an exe/file, or kill it (`closeApp` → `killApp`) |
| | `text` | K M | clipboard + Ctrl+V (+ Enter) |
| OBS (obs-websocket v5, `obs-websocket-js`) | `record-start-stop` `record-continue-pause` `stream` `virtual-camera` `replay-buffer` `replay-buffer-save` `scene-collection` `scenes` `source-visibility` `obs-profile` `audio-mixer` `media-source` `studio-mode` `transition` `filter` `screenshot` `scene-transition` | K M (`audio-mixer` also S R D N) | `ToggleRecord`, `ToggleRecordPause`, `ToggleStream`, `ToggleVirtualCam`, `SetCurrentProgramScene`, `SetSceneItemEnabled`, `SetInputVolume{inputUuid, inputVolumeDb}` (fader maps 0..100 linearly onto `volumeRange`, default −96..+26 dB; encoders ±1 dB), `SetInputMute` (hold-to-mute/talk modes 2/3 act on release) … The connection is auto-filled from `%APPDATA%/obs-studio/plugin_config/obs-websocket/config.json`. |
| Twitch (tmi.js chat + Helix) | `chat-message` `clear-chat` `emote-chat` `followers-chat` `sub-chat` `slow-chat` `shield-mode` `play-ad` `stream-game-title` `create-stream-marker` `create-clip` `open-last-clip` | K M | OAuth via `http://localhost:4040/auth/callback/twitch` |
| Streamlabs Desktop (SockJS `http://127.0.0.1:59650/api` + token) | `streamlabs-record` `streamlabs-stream` `streamlabs-scenes` `streamlabs-source-visibility` `streamlabs-audio-mixer` `streamlabs-studio-mode` `streamlabs-transition` | K M (`audio-mixer` also S R D N) | `setDeflection 0..1`, `setMuted` … |
| Discord (RPC over local IPC, OAuth `rpc.voice.*`) | `discord-mute` `discord-deafen` `discord-voice-channel` `discord-text-channel` `discord-set-audio-device` `discord-voice-input-mode` `discord-user-volume-control` `discord-output-volume-control` `discord-input-volume-control` | K M (volumes also S R D N) | `setVoiceSettings`, `SET_USER_VOICE_SETTINGS`. Volumes are 0..200, with fader = `value*2` (input volume uses `value`) |
| MasterCTRL (`CMAgent.exe`, worker `Ctrl.js`) | `ctrl-lighting-profile` `ctrl-cooling-profile` `ctrl-scenes` | K M R D N | next/prev profile in Cooler Master's PC-hardware app. Not relevant on Linux. |
| Photoshop / Illustrator / Premiere | ~100 `ps-*`, `ai-*`, `pr-*` | K (undo/zoom/brush/blend also R D N) | Mostly preset hotkeys. `ps-hue`, `ps-hue-saturation`, `ps-lightness`, `ps-flatten-image` and tool selection write a JSX file and run `Photoshop.exe -r` (`run-photoshop-script`). |

Encoder and roller dispatch (`executeKnob3Action`, `executeRollerAction`, `executeDialEncodeAction`) is the same switch, reduced to step-able actions: hotkey family, brightness ±5, lighting effect next/prev, profile next/prev, scroll, volume ±5, OBS ±1 dB, Streamlabs, CTRL, Discord volumes, PS hue/sat/lightness.

### 5.3 Storage layout (`%APPDATA%/MasterHub`, `GET_MASTERHUB_DIR`)

| File | Content |
|---|---|
| `profiles.json` | `[{id:int, value:name, label:name, editstatus, serialNumber:<Base serial>, selected}]`. **Profiles belong to a Base serial.** |
| `hubConfig.json` | `hubProfileMap`: `{<Base serial>: {name:"DeviceN", inUseProfileId}}` |
| `profile<id>_<BaseSerial>` (no extension) | the profile JSON (§5.4) |
| `uploadImage/` | user-uploaded images/GIF copies (`<timestamp>.gif`) referenced by `gifPath` |
| `gifPool/<md5>` | pre-rendered GIF frame cache (§4.1) |
| `keypadBuffer.json` | image buffer cache (worker `keypadBuffer.js`) |
| `masterHubConfig_profile_<name>.mhz` | export staging |
| `systemSetting/` | watched; any change saves and puts the LCDs in hardware mode |

The renderer also uses `localStorage` for: `obsConnection {obsIp, obsPort, obsPwd}`, `streamlabsConnection`, Twitch and Discord tokens, `lang` and `isAutomaticallyStart`.

### 5.4 Profile JSON (`store/index.js syncConfig`, line 1440)

```jsonc
{
  "serialNumber": "<Base serial>",
  "profileId": 3, "profileName": "Default",
  "keypad":        { "<M4315 serial>": [ /*page*/ [ /*15 buttons*/ {…} ] ] },
  "keypadBtnConfig": { "<M4315 serial>": { "currentPage":0, "viewMode":"master", … } },
  "knobsList":     { "<M003 serial>":  { "rotateR":{…}, "rotateG":{…}, "rotateB":{…} } },
  "rollerBlock":   { "<M002 serial>":  { "left":{…}, "right":{…} } },
  "dialEncoder":   { "<M131E serial>": { … } },
  "sliders5List":  { "<M005S serial>": { "0":{…}, …, "4":{…} } },
  "touch46s": {}, "touch50s": {},
  "deviceLightingEffect": { … },
  "colorPickerFavoriteColors": [ … ], "iconLibrarys": [ … ],
  "devices": { … }, "filtered": true
}
```

- Node `Buffer`s (images, frames) are serialized as `{"type":"Buffer","data":[…bytes…]}` and revived on import (`importConfig`, line 1530). Profiles are therefore large.
- Module configs are keyed **by module serial**, not by slot. `filterConfigdataByParentSerialNumber` drops entries for modules that are attached to a different Base. If nothing would be left, it keeps everything, so a profile can be applied to a "new" module serial. When an unknown module appears, the UI offers to apply an existing configuration ("New product module detected…", `getAllPreviewData`).

### 5.5 `.mhz` export/import

- `.mhz` is a plain **ZIP** (node `archiver`, zlib level 9; main `compressProfiles`). It contains:
  - `profile<id>_<BaseSerial>`: the JSON above, at the archive root
  - `uploadImages/<file>`: images referenced by `gifPath`
  - `gifPools/<md5>`: frame caches
- A multi-profile export is a `masterHubConfig_profiles.zip` holding several `.mhz` files.
- Import (`extractProfiles`): 7-Zip extracts into `profileExtract/`. The first entry matching `/^profile[0-9]+_(.*)/` is the profile. `uploadImages/*` and `gifPools/*` are copied into the data dir unless already present.
  - It requires numeric `profileId` and `profileName` (or `profilename`).
  - A name clash gets the suffix `_copy`, `_copyN`.
  - The result is attached to the current Base.
- Bundled templates: `resources/masterHubConfig_profile_Default.mhz` and `masterHubConfig_profile_Photoshop.mhz`.

### 5.6 System info (LCD "CPU/GPU temperature/usage" modes)

`hub_daemon/main.js` is a separate Electron process that hosts a socket.io server on `:3636`. It calls `addonStack.initCpuid(cb)` and polls `fetchSysInfo()` every 1 s. The GUI sends `"message"` once a second and the daemon replies with `{cpuUtilization, cpuTemperature, gpuUtilization:[…], gpuTemperature:[…]}`. The renderer maps that into `systemRTInfo.cpuLoad/cpuTemp/gpuLoad/gpuTemp`, using GPU index 0, and the dial LCD redraws every 1 s. On Linux use hwmon/`/proc/stat` plus the NVML/amdgpu sysfs equivalents directly.

---

## 6. Hardware mode vs software mode (standalone behaviour)

What the code shows:

- `Set_Sw_Mode` exists only for the **LCD modules**: M4315 and M131E (and, by SDK name, M3700/M4600-class screens). The M0-family modules (Base, slider, knobs, roller) have no mode switch.
- **Software mode (true)**: the host owns the screens. The app pushes every key image and the dial LCD content, and nothing is cached across page switches.
- **Hardware mode (false)**: the app sets this whenever it is *not* driving the screens: during the intro animation, on system suspend, and on exit or update. So hardware mode means "firmware drives the display on its own".
  - The SDKs contain `Test_Animation_Recovery_Process_CM_M4315`/`_M131E` and `Save_Screen_Picture_CM`. The intro view replays a 1459-frame M4315 and a 25-frame M131 animation. Together these strongly suggest the firmware plays a built-in boot/idle animation or logo in hardware mode.
  - Unverified: whether hardware mode also suppresses input reports.
- The modules' HID descriptors are vendor-defined (usage page `0x0C`/`0x01` with `0xFF00` fields; see `PROTOCOL.md`). They expose no keyboard or consumer-control usages, and the app never writes a key map to flash. **Without host software, keys, faders and knobs do nothing on the PC.** All actions (hotkeys, volume, OBS…) are executed host-side.
- ARGB LEDs keep running the last mode, colour and brightness that was sent, because the effect runs in the module MCU. The app always passes `saveFw=false`, so whether settings survive a power cycle depends on firmware defaults. The default seen in the app is rainbow, `#743ADF`, brightness 50.
- LCD backlight levels are likewise only set at runtime.

Suggested Linux behaviour:

- On start: enumerate, then `Set_Sw_Mode(true)` on every M4315 and M131E, then push the current page.
- On exit, suspend (logind `PrepareForSleep`) or crash-safe shutdown: `Set_Sw_Mode(false)`, so the device falls back to its own animation instead of showing stale images.
- On resume: wait about 2 s, re-enable software mode and re-push everything, as the vendor app does.

---

## 7. Implications for the Linux app (summary)

1. **Discovery.** Use udev/sysfs (or libusb) and match VID `2516` against **both** PID sets. For each Base, find the topmost `067b:2586` ancestor that has the Base MCU at least 2 levels below it. The port suffix below that hub, joined as digits, is the slot key. Look up `(module type, port string)` → (cells, rotation) using §2.3. Group by Base. Re-scan on udev add/remove instead of polling every 2 s.
2. **Input.** One reader per hidraw node, blocking or epoll. Do not poll at 20 ms with single reads. Keys are edge-triggered (press **and** release: hotkeys are held). Faders are absolute 0..100, so diff them to find the moved fader. Encoders and rollers send ±1 per report. Dial long-press is ≥600 ms.
3. **Output.** Render 72×72 (key) and 240×240 (dial) **JPEG** and send them via the image command in `PROTOCOL.md`. No RGB565. Keep about 10 fps for animated content. Use a global backlight value of 0..100 and the ARGB mode/colour/brightness from §4.2. Resend the ARGB state periodically, or at least on hot-plug.
4. **Actions.** Map nut-js key codes to evdev, use uinput for keys and scroll, and PipeWire/PulseAudio for per-app volume (instead of nircmd). OBS uses obs-websocket v5 and Discord uses RPC over `$XDG_RUNTIME_DIR/discord-ipc-0`. Streamlabs and MasterCTRL are Windows-only, and the Adobe JSX scripting is too.
5. **Profiles.** Keep the per-Base, per-module-serial JSON model. For compatibility, read `.mhz` (ZIP with `profile<id>_<sn>` + `uploadImages/` + `gifPools/`) and revive `{"type":"Buffer"}` blobs. A cleaner native format can be exported alongside it.
6. Do not copy these vendor bugs: the `rootAddress` sum, the dead hub `rootAddress` check, `saveFw: false.serialNumber` in the Base hot-plug path, and M5000 using `connectM4600`.
