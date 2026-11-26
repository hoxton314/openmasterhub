# Cyberpunk 2077 game mode

- `pulpit_bridge/` — CET mod. `init.lua` is the tiny core (file I/O, command loop);
  `logic.lua` holds stats + commands and reloads with the `reload` command, so changes need no
  CET "Reload all mods". Deploy with `tools/deploy.sh` (copies; CET may not follow symlinks).
- Files in the mod folder: `stats.json` (4×/s), `cmd.txt` (`<seq>\t<cmd>\t<arg>` lines, seq
  only grows; written by PULPIT), `result.json` (last 10 results).
- `tools/cpb.py CMD [ARG]` sends a command and prints the result; `cpb.py eval 'return …'`
  runs Lua in the game (dev). `tools/hubkey.py COMBO` sends keys through masterhubd
  (`o@800` = hold 800 ms). The game must be focused for keys.
- Window class: `steam_app_1091500`.

Tested in game 2026-11-16 (patch 2.5): stats (HP, RAM, eddies, level, cred, district,
wanted, speed, weapon + mags + reserve, time), eddies, heal, ram, ammo, wanted (cleared 2★),
weather cycle, stamina toggle, waypoint (own pin, else tracked quest marker) + back,
key tap (M) and hold (O = time skip). Present but not yet fired: level, cred, perk, attr,
relic, god.

## Icons (spawn panel / build editor)

- WolvenKit CLI 9.1.1 (Linux) in ~/Tools/wolvenkit/9.1.1. Its texture export needs a Windows-only
  library (DirectXTexNet), so textures are uncooked serialized (`-s`) and decoded by
  `icons/xbm2png.py` (raw BC7/BCn → DDS header → Pillow; UI textures are stored bottom-up: flip).
- Atlases: `uncook … -r '\.inkatlas$' -s` (content, ep1, mod) + the textures they point to,
  into ~/.local/share/pulpit/cp-icons-src (≈3 GB).
- Bridge `dump_icons` → icons.json (UIIcon records: atlas FNV1a64 hash + part; item hints:
  iconPath, appearanceName, icon record, appearanceResourceName + first visual tag; perk icons).
- `icons/build.py` resolves items → icons (weapons: weapon_<appearanceResourceName>_None__<tag>;
  clothes: clothing_…_item_<appearanceName>_<gender>_; cyberware: iconPath) and cuts PNGs to
  ~/.local/share/pulpit/cp-icons + index.json.
