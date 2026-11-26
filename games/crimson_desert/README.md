# Crimson Desert game mode

Keys only, no mods (2026-11-23, game 2.05 "Enhanced", Steam 3321460 under Proton).
Definition: `crates/pulpit/src/games/crimson_desert.rs`. Window class: `steam_app_3321460`.

- Switch off: `[games] crimson_desert = false` in pulpit.toml, then restart pulpit.
- Remove for good: delete that .rs file, its line in `games/mod.rs` (`ALL`) and this folder.
- Binds are the game's "Default" keyboard preset (Input screen). On a controller the menu
  shortcuts are unassigned, so the hub keys are the short way to map, inventory, journal,
  skills and photo mode. `1` `2` `3` = mount / weapon / food (tap; hold opens the quick slot).
- `tools/map.sh`: interactive web map on workspace 8 of DP-3 (`PULPIT_CD_MAP` overrides the address).
- Look at the keys without the hub: `pulpit sheet crimson_desert out.png`.

## Look

Picked by the user on the proposals board (docs/boards/crimson-look.html, artifact
5ZPcqPEPt5nmuwtd8bCEEQ, 2026-11-23): plaque keys, IM Fell English SC, icons from the game,
heraldic colours (gold menus, crimson actions, bone tools), ember press, studs dial ring, ember intro.

- Renderer: `crates/pulpit-render/src/face/crimson.rs` (`Theme::Crimson`), intro in `intro.rs`.
- Icons: `crates/pulpit-render/art/game/cd-*.png` are grey masks cut from the user's screenshots
  (the game's artwork: never publish them); `art/*.svg` are game-icons.net drawings (CC BY 3.0).
  A key lists its pictures best first: `ka("INVENTORY", glyph, "cd-backpack|knapsack", …)`.
- All ten game keys now have the game's own icon (map = compass, mount = horseshoes, skills = the
  Spinning Slash skill icon). ELEMENT's flame is the small Fire Resistance stat icon: swap it when
  the save has a real element icon to screenshot.
  To add one: crop the icon square, grey levels to a 128 px mask, add it to `masks!` in `art.rs`.
- Board images again: `cargo run --release -p pulpit-render --example crimson_board docs/boards/crimson-look`.

## Mods (not used; state on 2026-11-23)

- No Lua, console or outside interface: a bridge like Cyberpunk's would need our own ASI plugin
  and breaks on every patch.
- Trinity mod menu (ASI, in-game overlay; god mode, warp, items, time of day): no build
  confirmed for 2.05. If added later: its own keybinds cover only opening the menu and free
  flight, so the hub could open the menu and steer it (dial = up/down, press = select).
- CDUMM (data mods) cannot install on game 2.04+.
- ASI under Proton is reported to need `WINEDLLOVERRIDES="winmm=n,b"` in the launch options.
