# OpenMasterHUB

Open-source Linux driver, service and desktop app for the **Cooler Master MasterHUB** modular
controller. No vendor software needed.

Supported modules (all verified on hardware): Base, 15-key IPS keypad, 5 faders, 3 knobs,
2 rollers, and the dial with a 240×240 screen.

## Components

| Crate | What it is |
|---|---|
| `masterhub-proto` | Wire protocol: command encoding and input decoding, no I/O |
| `masterhub-core` | Config model, Base slot layout, face/widget rendering, D-Bus contract |
| `masterhubd` | Background service: drives the hardware over hidraw, profiles/pages/folders, live widgets, per-app profiles (Hyprland), D-Bus API |
| `openmasterhub` | GTK4 + libadwaita app: live picture of your hub, click-to-configure editor |

Protocol documentation: [`docs/PROTOCOL.md`](docs/PROTOCOL.md) and `docs/protocol-*.md`.

## Install

### Arch Linux (AUR)

```sh
paru -S openmasterhub-bin        # or yay -S openmasterhub-bin
systemctl --user enable --now masterhubd
```

The package installs the udev rule (access for the logged-in user), the systemd user unit and
the app launcher entry.

### From source

```sh
# Access to the modules for the logged-in user
sudo install -m644 udev/70-masterhub.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger

# Build and install the binaries to ~/.cargo/bin
cargo install --path crates/masterhubd
cargo install --path crates/openmasterhub

# Run the service at login (the unit expects /usr/bin; point it at ~/.cargo/bin)
mkdir -p ~/.config/systemd/user
sed "s|/usr/bin/|$HOME/.cargo/bin/|" dist/masterhubd.service > ~/.config/systemd/user/masterhubd.service
systemctl --user enable --now masterhubd

# App launcher entry
install -Dm644 dist/org.openmasterhub.OpenMasterHub.desktop ~/.local/share/applications/org.openmasterhub.OpenMasterHub.desktop
```

Virtual key presses use `/dev/uinput`, which the logged-in user can normally access. Volume
widgets use `wpctl` (PipeWire) and GPU widgets use NVML (NVIDIA) or sysfs (AMD).

## Configure

Open **OpenMasterHUB**, click a key, knob, fader, roller or the dial, and edit it. Changes
apply to the hardware immediately. The configuration lives in
`~/.config/openmasterhub/config.toml`; see [`examples/config.toml`](examples/config.toml) for
every option. The service reloads the file when it is edited by hand.

## Notes

- After the vendor app (or anything else) switches the keypad/dial to hardware mode, they
  ignore software mode until the MasterHUB is unplugged and replugged. `masterhubd` never sends
  hardware mode and tells you in its log when a replug is needed.
- `tools/` contains the Python probe and usbmon scripts used while reverse-engineering.
- Releases and the AUR package are built by CI; see [`docs/RELEASING.md`](docs/RELEASING.md).
