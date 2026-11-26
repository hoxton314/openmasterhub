# Live surface

Other programs can take over hub controls through the service's local socket. These are
newline-delimited JSON requests, the same as the rest of the IPC in `docs/app-architecture.md`.
A client claims controls, draws its own keys and dial, sets LEDs and gets the inputs. The
config keeps working under it; when the claim ends, the config's faces and actions come back.

```json
{"id":1,"method":"claim","params":{"name":"pulpit","controls":{"keys":[0,1,2,3,4],"dial":true,"knobs":[0],"leds":["base",{"knob":0}]},"apps":[],"priority":0}}
→ {"type":"response","id":1,"result":{"claim":7}}
{"id":2,"method":"frame","params":{"claim":7,"target":0,"jpeg":"<base64 72×72 JPEG>"}}
{"id":3,"method":"frame","params":{"claim":7,"target":255,"jpeg":"<base64 240×240 JPEG>"}}
{"id":4,"method":"led","params":{"claim":7,"zone":"base","mode":"breathing","color":"#a891ff","brightness":100}}
{"id":5,"method":"release","params":{"claim":7}}
```

- **Controls:**
  - `keys` 0..=14 (row-major);
  - `dial` (screen, turns, press);
  - `knobs` 0..=2;
  - `rollers` (0 left, 1 right);
  - `faders` 0..=4;
  - `leds`: `"base"`, `{"knob":i}` or `{"fader":i}`.
- **When it applies:** `apps` is a list of window-class globs, as for profiles. The claim
  applies only while one of them is focused, or always when the list is empty. A pattern
  `class@title` (both globs) also matches the window title, for apps that share a class
  (games started through one launcher); titles come from the Hyprland backend only so far.
- **Overlaps:** the higher `priority` wins, then the newer claim. A claim that gets its
  controls back shows its last frames again.
- **While owned:** the service doesn't draw the control or run its configured action. Its
  `input` events carry `"claim": id`.
- **`claim` events:** they tell each client what it owns right now (`owns`, `active`), and how
  far to turn images clockwise so they read upright (`key_turn`, `dial_turn`).
- **Frames:**
  - a key or dial that is claimed but has no frame yet shows blank;
  - only the newest frame per target reaches the module, so pushing faster than the hardware
    takes them doesn't build up lag;
  - the app's live picture gets at most 10 updates per second per target.
- **LEDs:** fields left out keep their value; a colour on its own means a static colour.
  Modes: off, static, breathing, strobe, spring, cycling, random, wave, water, rainbow.
  The Base status light (Claude Code) pauses while a client owns the Base LED.
- **End of a claim:** it ends with `release` or when the connection closes.

Measured on hardware (2026-11-15): one client streaming 5 keys plus the dial reached
53 rounds per second. The keypad takes about 600 key images per second in total, the dial
about 68 frames per second.

## Running actions

`run_action` runs any config action now, as if a control bound to it fired. A client that owns
a key uses it to get the service's key presses, volume, per-app volume, commands, page and
profile switches and Claude session jumps, without implementing them itself:

```json
{"id":6,"method":"run_action","params":{"action":{"keys":"ctrl+shift+t"}}}
{"id":7,"method":"run_action","params":{"action":{"volume":{"level":"+2","app":"discord"}}}}
{"id":8,"method":"run_action","params":{"action":{"session":0}}}
```

## Backlight

`backlight` dims the key and dial screens for as long as the claim lives: level 0..=100, or
`null` for the config's level. When the claim ends, the config's level comes back, so a client
that quits never leaves the screens dark.

```json
{"id":9,"method":"backlight","params":{"claim":7,"level":5}}
```
