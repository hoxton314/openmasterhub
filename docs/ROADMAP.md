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

## Next: finish Windows

1. **W2, widgets** (`crates/masterhubd/src/stats.rs`). Today these are Linux-only:
   - `volume`: shells out to `wpctl`. Windows: Core Audio, `IAudioEndpointVolume` on the
     default render endpoint (level + mute).
   - `now_playing`: MPRIS over D-Bus. Windows: SMTC,
     `GlobalSystemMediaTransportControlsSessionManager` (title, artist, thumbnail stream).
   - `cpu_temp`: hwmon sysfs. Windows has no reliable unprivileged source
     (`MSAcpi_ThermalZoneTemperature` needs admin and is often fake). Options: read
     LibreHardwareMonitor's WMI namespace when it is running, or show "n/a".
   - `gpu_usage` / `gpu_temp`: NVML through `nvml-wrapper` should already work (it loads
     `nvml.dll`); verify on real hardware. AMD on Windows: ADLX, or skip.
2. **Volume actions that work on Windows.** The examples and the app's placeholder use
   `wpctl`. Add built-in actions (e.g. `volume = "+2"`, `mute = "toggle"`) backed by PipeWire on
   Linux and Core Audio on Windows, so configs are portable. Later: per-app volume
   (`ISimpleAudioVolume` / PipeWire nodes), the vendor app has this too.
3. **Verify on a real Windows PC** (the VM cannot do these):
   - socket (slot) detection from `DEVPKEY_Device_LocationPaths`: needs the Base's internal
     Prolific hubs to be visible, which a VM hides. The app's picture of the hub depends on it.
   - installer + autostart + update over an existing install.
   - focus profiles for common exes (`msedge.exe`, `chrome.exe`, `Discord.exe`, games).
4. **Code signing** for the installer (SmartScreen warns today). Needs a certificate decision
   from the maintainer: a paid OV/EV certificate or Azure Trusted Signing. Then sign in
   `build-windows.yml`.
5. **Windows default config / first run**: a starter profile that uses Windows actions instead
   of `wpctl`/`playerctl` commands.

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
