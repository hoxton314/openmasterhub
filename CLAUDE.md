# OpenMasterHUB — notes for agents

Open-source driver, service and app for the Cooler Master MasterHUB. Read `README.md` for the
layout, `docs/ROADMAP.md` for what to do next, `docs/PROTOCOL.md` for the wire protocol and
`docs/RELEASING.md` for releases.

## Hardware rules

- **Never send hardware mode** (`03 3A 00` keypad, `03 49 00` dial), not even in tests. The
  firmware then ignores software mode until the MasterHUB is unplugged and replugged.
- If the keypad or dial looks dead (logo animation still running, empty GET `0x03` after the
  software-mode SET), ask for a replug before debugging anything else.
- The vendor **MasterHUB** app must not run at the same time as `masterhubd`; it fights over the
  devices and can leave modules in hardware mode.
- Keep real device serial numbers out of committed files, docs and test fixtures.
- The keypad is 5×3 keys of 72×72 px (indices row-major); the dial screen is 240×240.

## Building

```sh
cargo clippy --locked --all-targets -- -D warnings   # CI runs this on Linux and Windows
cargo test --locked
cargo run -p masterhubd -- --list                     # list connected modules
cargo run -p masterhubd -- --config path.toml --check # validate a config
cd app && npm ci && npx tauri dev                     # the app against a running masterhubd
cd app && npm run dev                                 # UI only, in a browser with a mock hub
```

The app crate is `openmasterhub-app`. `tauri-build` requires the sidecar file
`app/src-tauri/binaries/masterhubd-<host-triple>[.exe]` to exist; for clippy/dev an empty file is
enough (CI touches one), for an installer copy the real `masterhubd` there.

### Windows

- Rust stable (MSVC toolchain), Node 22, WebView2 (preinstalled on Windows 11).
- Config: `%APPDATA%\openmasterhub\config.toml`. `cmd` actions run through `cmd /C`.
- The installed service autostarts from HKCU Run; end `masterhubd.exe` in Task Manager
  before running a dev build, so two services do not drive the modules at once.
- Installer without a release: Actions → *Build Windows installer* → Run workflow.

## Conventions

- Platform code lives behind `cfg` in `crates/masterhubd/src/{hid,keyboard,focus}/{linux,windows}.rs`
  and small `#[cfg(windows)]` functions (`stats.rs`, `actions.rs`). Keep Linux behaviour unchanged.
- Match the existing style: short doc comments, `anyhow` errors, `log` for diagnostics.
- PRs go to `main`; the maintainer merges. Releases are tag-driven with a manual approval gate.
