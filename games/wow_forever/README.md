# WoW Forever game mode

Started 2026-11-24. Definition: `crates/pulpit/src/games/wow_forever/` (`mod.rs` pages and
binds, `logs.rs` combat/chat log reader). Battle.net under Lutris (GE-Proton), install
`~/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_` (product `wowf`).

- Switch off: `[games] wow_forever = false` in pulpit.toml, then restart pulpit.
- Remove for good: delete the module folder, its line in `games/mod.rs` (`ALL`) and this folder.
- Window: class `steam_app_battlenet` (umu GAMEID of the whole Battle.net prefix: launcher, Diablo IV and WC3 share it), title `World of Warcraft`; the claim uses the engine fork's `class@title` pattern.

## Live data: the add-on's screen strip (2026-11-24)

WoW Forever writes its combat log in ~300 KB batches every few minutes and the chat log not at
all, so files cannot carry live data. The user chose a screen strip: our add-on draws a row of
6x6 px cells in the top-left corner (5 times a second): level, XP, XP max, rested, money,
uiMapID, position, combat/dead/resting/mounted, health/power/target (blocked by the game's
"secret values": 0), cooldowns of bar 1 buttons 1-12 and bar 2 buttons 1-6. The hub reads only
that 400x14 corner with grim while the game has focus (`strip.rs`: markers + checksum), drives
the dial (XP %, XP/h, gold/h, zone + live coordinates, fight timer, auto FIGHT page) and the bar
keys' cooldown sweeps, and writes `~/.local/share/pulpit/wow-live.json` for the panel.
`/pulpitstrip off|on` hides it and lists blocked fields. Every field is computed on its own
(pcall): a secret value blanks only that field.

## Rules (online game)

- One hub key = one of the game's own binds, pressed once. No chains, no timing tricks.
- Nothing reads the game's memory. Live data: our add-on's screen strip (user's choice, see
  above), plus the files the game writes (combat log, add-on save). Display only: nothing goes
  back into the game.

## Binds

Read at pulpit's start: the client defaults (`DEFAULTS` in mod.rs) with the newest
`WTF/Account/**/bindings-cache.wtf` on top. Restart pulpit after rebinding in the game. Keys
whose action has no bind show NO BIND (bar 2 = travel slots, bar 1 buttons 10-12): the PULPIT
add-on is meant to bind them.

## Pages (left roller)

PLAY (windows, bar 1-5, quests, autorun, mount, screenshot, clip) · FIGHT (bar 1 buttons 1-12,
target, sheathe, clip; comes up by itself in a fight, from the combat log) · TRAVEL (bar 2
slots: mount, hearthstone, 3 more; autorun, sit, walk, follow) · MENUS (every window) ·
CAPTURE (screenshot, hide UI, clip/replay/record, time, machine, music).

## Dial (right roller)

In a fight: DPS, fight time, damage taken. Otherwise: zone (+ last accepted quest), experience
this session and XP/h (rested bonus, level-up), gold looted and gold/h (+ last loot), last
fight, session clock. Needs `/combatlog` and `/chatlog` switched on in the game.

## Look (round 2 picks, 2026-11-24)

`Theme::Wow` (crates/pulpit-render/src/face/wow.rs): action-button keys (the game's icons, your
bind as the bar's hotkey text), the Cast press (yellow flash + cooldown sweep), the XP-bar ring,
and the zone card as the minimap over the current zone's own map. Intro: Hearthstone cast.
Board: https://claude.ai/artifact/Kn47A3coyLqzAXe8syuEpc (source docs/boards/wow-round2.html,
images from `cargo run --release -p pulpit-render --example wow_board docs/boards/wow-look`,
gitignored: game artwork).

Game files are read at run time from `~/.local/share/pulpit/wow-src` (tools/extract.py:
default set, `--zones` = all 50 zone maps + maps/zones.json, `--fdid N` = an icon by FileDataID,
`--find REGEX`). Never commit or publish them.

## The PULPIT add-on

`addon/PULPIT` (install with `addon/install.sh`, only with the game closed; `--remove`). On
login it binds free keys for the hub's bars (bar 1 buttons 10-12 = Ctrl+0 / Ctrl+- / Ctrl+=,
bar 2 buttons 1-6 = Alt+1-6; only keys nothing uses) and saves bars (icon FileDataIDs, names,
base cooldowns), level / XP / rested / money / zone / quests as JSON in `PulpitDB` (account
SavedVariables, written on /reload, logout, exit). The hub (`saved.rs`) reads it, puts the
spells' icons and names on the bar keys (extracting missing icons in the background), runs
cooldown sweeps from the combat log, and fills the XP and gold cards. `/pulpit` re-binds and
re-snapshots (visible after /reload). Keys it binds: bar 1 buttons 10-12 take Ctrl+0/-/= or,
when the game uses those (it does), Alt+Shift+1-3; bar 2 buttons 1-6 Alt+1-6. The hub takes its
bar binds from the save (what the game really has bound), never from guesses.

Screen layout (2026-11-24, user asked for a pleasant one): Edit Mode "Modern" preset kept for
bars, unit frames, minimap, buffs. Our add-on places the rest a moment after login: the chat at
bottom-left 440x200, the RestedXP guide window right above it (left column), top-left free for
party frames. `/pulpit layout` dumps every Edit Mode system and frame rect into the save;
`/pulpit layout off|on`.

Add-on set-up done in the game: Questie objective style = Questie; RestedXP guide = Speedrun
1-22 (6-13 Tirisfal Glades); Guidelime window hidden (`/lime` shows it; free 1-60 guides for
after 22); combat and chat logging on.

Driving the game for set-up (only with the user's go-ahead): ydotoold + `hl.dsp.cursor.move`
for clicks (WoW needs a small ydotool move before a click to register), `ydotool type` for chat
lines; the game window is title "World of Warcraft" on HDMI-A-2 (scale 1.2).

## Board

Round 1 answers: https://claude.ai/artifact/5QcCkrdyXmxNtnbeLDzE2Y (source
docs/boards/wow-round1.html). Paladin main (John), keyboard + mouse (controller sometimes),
levelling/dungeons/professions/AH/alts/exploring, no goals tracker. Panel = levelling helper
(zone map, current quest, class guides). High expectations on look and animation.

## Installed add-ons (2026-11-24)

Into `_forever_/Interface/AddOns/` (was empty; WTF/ untouched). Zips kept in
`~/Downloads/wow-addons-2026-11-24/`. Client build 1.60.1.71037, Interface 16001.

| Folder | Version | Loads in Forever via | Source |
|---|---|---|---|
| Questie | 12.13.0 | `Questie_Forever.toc` (16001) | GitHub Questie/Questie release `bundle/v12.13.0+v1.1.5` (only the Questie folder taken) |
| QuestieDB | 1.1.5 | `QuestieDB_Forever.toc` (16001) | GitHub Questie/QuestieDB `QuestieDB-1.1.5-Forever.zip` |
| Guidelime | 6.026 | `Guidelime-Camelot.toc` (16001; hard-depends on Questie) | GitHub max-ri/Guidelime v6.026 |
| RXPGuides (RestedXP, free part) | v4.11.40 | plain `RXPGuides.toc` (Interface list includes 16001) | GitHub RestedXP/RXPGuides v4.11.40 |
| TomTom | v4.4.1-release | plain `TomTom.toc` (list includes 16001) | WoWInterface id 7032 (MD5 matched) |
| Leatrix_Plus | 1.60.37 (forever build) | plain `Leatrix_Plus.toc` (16001) | CurseForge file 9744906 via mediafilez.forgecdn.net |
| Auctionator | 352 | plain `Auctionator.toc` (list includes 16001) | Wago (forever release, supports 1.60.1) |

Not installed: QuestieTrace (ships in the Questie bundle, not in the approved set).
Note: Auctionator's current version label is `352`; the "12.1.5" noted earlier was a
retail game patch, not an add-on version.

## Levelling helper panel (2026-11-24)

`tools/panel.py` (:8278) + `panel.html`, opened by `tools/panel.sh` on workspace 8 of the monitor
without the game (the hub runs it when the game starts). Zone map (live from MAP_CHANGE in the
combat log), your position + quest log from the add-on save (as of the last /reload), quest
pins from the QuestieDB export (`tools/questie/build.sh --faction Horde --class Paladin`, output
in wow-src/questie), class spells to train next from `tools/class_spells.py` (game tables via
wago CSV), guide links. Several characters: the strip carries class and faction, the panel picks the
QuestieDB export for that pair (`wow-src/questie/<Faction>-<CLASS>/`) and builds a missing one by
itself in the background (~4 s); after a character switch it asks for a /reload before showing
the quest log (the add-on's save is only written then). Kill the server: `kill $(lsof -t -iTCP:8278 -sTCP:LISTEN)`.
