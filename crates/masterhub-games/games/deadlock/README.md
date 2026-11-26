# Deadlock game mode

Keys only (2026-11-23). Definition: `crates/pulpit/src/games/deadlock.rs`. Steam 1422450 under
Proton, process `deadlock.exe`, window class `steam_app_1422450` (not yet seen running).

- Switch off: `[games] deadlock = false` in pulpit.toml, then restart pulpit.
- Remove for good: delete that .rs file, its line in `games/mod.rs` (`ALL`) and this folder.
- Binds are read at pulpit's start from the game's own file,
  `~/.local/share/Steam/userdata/<id>/1422450/remote/cfg/citadelkeys_personal.lst`
  (action → Key / Modifier). After rebinding in the game, restart pulpit.
- Rule for this game (online, anti-cheat): one hub key = one of the game's own binds, pressed
  once (SCORES holds Tab 1.5 s). No chains, no timing tricks, nothing that reads the game.
- Pages (left roller): LIVE first, a display page for Street Brawl where no game key is needed
  (time, session, date, CPU, GPU temperature / load / VRAM, music player with the track name,
  clips; readouts from `crates/pulpit/src/live.rs`, the machine and the player only), then
  EXTRA (console commands, see below), MATCH (shop, quick buy, scores, chat, ability upgrades), SHOP (tabs, search, tiers),
  REPLAY (pause, speed, targets, camera, whose vision).

## Look

Picked by the user on the proposals board (docs/boards/deadlock-look.html, artifact
6GRhXebk5x3D29StN9npcz, 2026-11-23): item-card keys, soul-burst press, soul-counter dial, emblem
intro. Built from the game's own files, nothing redrawn by hand:
- Colours: the `@define` names in the game's style sheets (panorama/styles/*.vcss_c in pak01):
  offWhite, soulColor, deadlockGreen*, weapon / spirit / fortitude sets, builds blue, team colours.
- Font: Retail Demo Bold, read at start from the install (`panorama/fonts/retaildemo-bold.otf`);
  not copied into the repo. Without the game installed Chakra Petch stands in.
- Icons: the game's vector icons (panorama/images/**.vsvg_c), cut into masks by
  `tools/icons.py` → `crates/pulpit-render/art/game/dl-*.png` (the game's artwork: never publish).
  All 654 extracted SVGs for browsing: `~/.local/share/pulpit/deadlock-src`.
- Renderer: `crates/pulpit-render/src/face/deadlock.rs` (`Theme::Deadlock`): keys are shop item
  cards (colour field, off-white name plate); intro `IntroStyle::Emblem`.
- Each key carries its own colours (`GKey::tint`): green = souls and buying, cyan = ability
  points, orange / purple / green = weapon / spirit / vitality, blue = replay, grey = our tools.

## Console commands (EXTRA page)

`cfg/pulpit.cfg` binds eight client-side commands to the navigation keys (Home, End, Ins, Del,
PgUp, PgDn, Pause, ScrollLock); the hub's EXTRA page presses those keys. Install or update with
`tools/install-cfg.sh` (copies the file, adds `exec pulpit` to autoexec.cfg; `--remove` undoes it).

Tested in the game on 2026-11-23 (build 6865, hideout):
- The game runs autoexec.cfg by itself and saves every bind it is given with the user's own
  (`userdata/<id>/1422450/local/cfg/user_keys_0_slot0.vcfg`): binds stay until unbound.
- A hub key reached the bind: Ins switched the game's FPS readout on.
- F13-F24 never reach the game (X turns them into media keys), so they cannot carry binds.
- `cl_hud_telemetry_ping_show` is saved by the game (1 = only when poor, the default; 2 = always).
- A game started by a script has no keyboard focus until its window is focused again; a game
  the user starts is fine.
- Not tried: the other seven commands (they need a match or a replay to show anything).
- MangoHud is not in Deadlock's launch options, so the FPS key uses the game's own readout.
