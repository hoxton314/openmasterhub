# Roadmap

Platform order: **Linux → Windows → macOS**. One GUI on every OS (Tauri + Svelte in `app/`),
one service (`masterhubd`) with a thin platform layer per OS. Linux behaviour must stay
identical while other platforms are added.

## Done

- **Linux** (0.1–0.2): every shipping module, profiles/pages/folders, live widgets,
  per-app profiles via Hyprland, Tauri app, AUR package.
- **Windows, phases W1, W3, W4** (0.3.0):
  - W1, service platform layer: hidapi (`hid/windows.rs`), `SendInput` (`keyboard/windows.rs`),
    per-app profiles by focused exe name (`focus/windows.rs`), `%APPDATA%\openmasterhub\config.toml`,
    `cmd /C` for `cmd` actions, CPU and RAM widgets.
  - W3: D-Bus replaced by a local socket / named pipe IPC (`masterhub-core/src/ipc.rs`), same
    Tauri app on both OSes.
  - W4: NSIS installer with the service bundled as a sidecar, per-user, HKCU Run autostart,
    built by CI and attached to the release.
  - Verified in a Windows 11 VM with the modules passed through (see `docs/PROTOCOL.md`, Windows).

- **Windows, W2 and the rest** (unreleased, not yet verified on hardware):
  - widgets: `volume` through Core Audio (`audio/windows.rs`), `now_playing` through SMTC with
    the thumbnail as cover art, `cpu_temp` and non-NVIDIA GPU widgets from
    LibreHardwareMonitor's (or OpenHardwareMonitor's) WMI namespace while it runs
    (`stats/windows.rs`).
  - built-in `volume = "+2" | "40" | "{value}"` and `mute = "toggle" | "on" | "off"` actions
    (`wpctl` on Linux, Core Audio on Windows), used by the examples and the app.
  - first run on Windows writes `examples/starter.toml`, which uses only built-in actions.
  - Claude Code status: the hook finds the `claude` process and the service drops sessions
    whose process exited, as on Linux.
  - the service log is capped at 1 MiB (one previous log kept).

## Next: finish Windows

1. **Verify on a real Windows PC** (the VM cannot do these):
   - the new widgets and volume actions above, with Spotify/a browser for now playing and
     LibreHardwareMonitor for temperatures; NVML GPU widgets on an NVIDIA card.
   - socket (slot) detection from `DEVPKEY_Device_LocationPaths`: needs the Base's internal
     Prolific hubs to be visible, which a VM hides. The app's picture of the hub depends on it.
   - installer + autostart + update over an existing install, and the starter config.
   - focus profiles for common exes (`msedge.exe`, `chrome.exe`, `Discord.exe`, games).
   - Claude Code hooks through Git Bash with the full path from `docs/CLAUDE_CODE.md`.
2. **Code signing** for the installer (SmartScreen warns today). Needs a certificate decision
   from the maintainer: a paid OV/EV certificate or Azure Trusted Signing. Then sign in
   `build-windows.yml`.
3. **Per-app volume** (`ISimpleAudioVolume` / PipeWire nodes); the vendor app has this too.

## Then: macOS

- HID through hidapi (IOKit backend); check whether macOS needs Input Monitoring permission
  for HID access.
- Key injection: `CGEventPost` (needs Accessibility permission).
- Focus: `NSWorkspace` frontmost application (bundle id).
- Widgets: CoreAudio volume, `MediaRemote` now playing (private framework) or skip, IOKit /
  SMC temperatures.
- Packaging: Tauri `.app`/`.dmg`, notarisation, LaunchAgent for the service.

## Anytime

- Action library: media controls without `playerctl`, per-app volume, OBS (websocket).
- Focus tracking beyond Hyprland on Linux: KDE (KWin script), GNOME (extension), Sway (IPC).
- Base tilt sensor: input `0x2B` reported once and then went silent; orientation currently comes
  from the config (`landscape` by default).
- Confirm which physical roller is "left" when the roller module is rotated.
