# Game modes

While a game has focus, its game mode takes over the keypad, the dial and both rollers. It
shows keys for that game, an intro when the game starts, and readouts on the dial. When you
leave the game, your own profiles come back. The modes run in a separate program,
`masterhub-games`, that talks to `masterhubd` through the [live surface](LIVE_SURFACE.md).

They come from Dawid Gorczyca's [PULPIT](https://git.kreatura.org/inqu/pulpit) and are
**opt-in**: every game stays off until you switch it on.

| Game | `setup` name | Keys | Extras |
|---|---|---|---|
| Cyberpunk 2077 | `cyberpunk` | the game's binds, combat page | cheats, stats on the dial, favourites and loadouts through a [Cyber Engine Tweaks](https://github.com/maximegmd/CyberEngineTweaks) mod |
| Deadlock | `deadlock` | your own binds, read from the game's key file | console commands on spare keys; a page with time, CPU/GPU and music |
| World of Warcraft (WoW Forever) | `wow_forever` | your binds and bars, with their spell icons and cooldowns | an add-on and a screen strip give XP, gold, zone and fight state |
| Crimson Desert | `crimson_desert` | menus, quick slots, clips | none (keys only) |

Online games (Deadlock, WoW) only press the game's own binds, one press per key. Nothing reads
the game's memory.

## Switching a game on

**In the app** (Windows and Linux): open **Game modes** in the sidebar and click a game. The
dialog shows:

- the game folder it found through Steam (**Browse…** picks another one: GOG, Heroic,
  Battle.net);
- every step, with its paths: the files it puts into the game (Cyberpunk's Cyber Engine Tweaks
  mod, Deadlock's `pulpit.cfg` and `autoexec.cfg` line, WoW's add-on), the icons it extracts
  from your copy and where they go, and turning the mode on.

Nothing happens until you click **Approve and turn on**. The same dialog turns a game off
again. The game-mode service then starts (on Linux, its systemd user unit is enabled), and it
reloads by itself whenever a game is switched on or off.

**From a terminal** the same steps are available:

```sh
masterhub-games list                      # games in this build, on/off, where they were found
masterhub-games plan deadlock             # what setup would do, as JSON (what the app shows)
masterhub-games setup cyberpunk --path "/games/Cyberpunk 2077"
masterhub-games disable cyberpunk
systemctl --user enable --now masterhub-games
```

## Game artwork stays on your machine

OpenMasterHUB ships nothing from any game: no icons, fonts or textures, and no screenshots. A
game's look is built from your own install, on your machine:

- **Deadlock:** `setup` cuts the game's vector icons out of `game/citadel/pak01_dir.vpk` into
  `~/.local/share/openmasterhub/games/deadlock/masks` (Windows:
  `%APPDATA%\openmasterhub\games\...`). The display font is read from the install when the
  mode starts.
- **WoW Forever:** the fonts, icons and maps are read from `games/wow_forever/src`. The game
  keeps them in Battle.net's CASC storage, and an extractor for that is not built in yet. The
  mode works without them, with its stand-in look (labels, Cinzel and IM Fell).
- **Cyberpunk 2077 and Crimson Desert** need nothing from the game. Their keys use Font
  Awesome glyphs and game-icons.net drawings.

Without the extracted files, every key falls back to a Font Awesome glyph. Setting a game up
again (the same dialog) extracts them anew, for example after a game update.

## Config

`games.toml` sits next to the service's `config.toml`
(`~/.config/openmasterhub/games.toml`, `%APPDATA%\openmasterhub\games.toml`):

```toml
[deadlock]
enabled = true
path = "/games/SteamLibrary/steamapps/common/Deadlock"   # optional, found through Steam
class = "steam_app_1422450"                               # optional: the window to follow

[clips]                # optional commands for the SAVE CLIP, REPLAY and RECORD keys
save = "obs-cmd replay save"
```

The window to follow is `steam_app_<id>` under Proton and the exe name on Windows. WoW under
Lutris uses `class@title` (`steam_app_battlenet@World of Warcraft`). `masterhub-games` reloads by itself when the file
changes.

## Building

Each game is a Cargo feature (`cyberpunk`, `deadlock`, `wow`, `crimson`), and all of them are
on by default. To build only some:

```sh
cargo build --release -p masterhub-games --no-default-features --features deadlock,crimson
```

`masterhub-games sheet GAME out.png` renders a game's pages as one picture, so you can check
the look without the hub.

## Platform notes

- **Window following** uses masterhubd's focus backends: Hyprland or KDE on Linux, the
  foreground program on Windows.
- **Window titles** (`class@title`) come only from the Hyprland backend so far.
- **WoW's live strip** is read with `grim` and `hyprctl` on Linux (Hyprland), and with GDI on
  Windows.
- **Readouts:** GPU readouts use `nvidia-smi`. CPU and the music player (`playerctl`) are
  Linux only for now.
