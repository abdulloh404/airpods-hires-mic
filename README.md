# AirPods Hi-Res Mic for Linux

AirPods Hi-Res Mic exposes the proprietary AirPods microphone stream as a
virtual Linux microphone while leaving Bluetooth playback on A2DP/AAC. It ships
as a Rust audio daemon, a battery diagnostic CLI, and a Tauri 2 desktop settings
application.

> [!IMPORTANT]
> This is experimental, unofficial software based on an undocumented AirPods
> protocol. It is not affiliated with or endorsed by Apple.

## Features

- Receives the AACP microphone stream without switching to HFP/HSP.
- Decodes AAC-ELD with FDK-AAC and creates the `AirPodsHiRes` virtual source.
- Scans Apple BLE advertisements for left, right, and case battery levels.
- Configures microphone gain and limiter from the desktop UI.
- Runs the microphone engine as a per-user systemd service.
- Keeps development and installed services/configuration completely separate.

## Project layout

```text
crates/airpods-core/       AACP, BLE battery, decoder, DSP, framing
crates/airpods-daemon/     audio daemon, virtual microphone, command-line tools
apps/desktop/              Tauri 2 + React + TypeScript desktop application
debian/                    Debian package metadata and user service
packaging/                 desktop entry, icons, templates and examples
scripts/                   development and package build helpers
docs/                      installation, development and packaging guides
```

## Install a Debian package

Download the `.deb` matching the machine architecture, then install it with
APT so runtime dependencies are resolved:

```bash
sudo apt install ./airpods-hires-mic_0.1.0_amd64.deb
```

On GNOME, install and enable the recommended **AppIndicator and
KStatusNotifierItem Support** extension before relying on close-to-tray
behavior.

Open **AirPods Hi-Res Mic** from the application menu. On first launch:

1. Connect the AirPods in the desktop Bluetooth settings.
2. Select the connected device in the application.
3. Save the microphone gain/limiter settings if desired.
4. Press **Start mic**.

The package installs the system-wide user unit, but the unit only runs after
the current user has selected a device. It does not pair devices or modify the
active Bluetooth profile, PipeWire configuration, A2DP, or AAC settings.

Check the installed service:

```bash
systemctl --user status airpods-hires-mic.service
journalctl --user -u airpods-hires-mic.service -f
pactl list short sources
```

## Battery CLI

Open the AirPods case near the computer and run:

```bash
airpods-battery
```

The scan stops after receiving a supported AirPods advertisement or after its
10-second timeout. Unencrypted advertisements normally expose battery values in
10% increments and may omit the case value.

## Development

Development uses `airpods-hires-mic-dev.service` and `dev.environment`; it never
controls the installed production service.

```bash
./install.sh --dev --device AA:BB:CC:DD:EE:FF
./scripts/dev.sh
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the complete workflow.

## Build a `.deb`

Install the build dependencies listed in `debian/control`, then run:

```bash
./scripts/build-deb.sh
./scripts/verify-deb.sh ../airpods-hires-mic_0.1.0_amd64.deb
```

Build on the oldest supported Linux baseline to avoid requiring a newer glibc
than downstream machines provide. See [docs/PACKAGING.md](docs/PACKAGING.md).

## Remove

Keep this user's settings:

```bash
airpods-hires-mic-remove --keep-config
```

Purge the package and this user's production settings:

```bash
airpods-hires-mic-remove --purge
```

The helper stops the current user's microphone service before invoking APT.
From a source checkout, `./uninstall.sh --package` and `--purge` provide the
same behavior.

Remove only the isolated development setup:

```bash
./uninstall.sh --dev
```

Package removal never changes Bluetooth pairings, the audio stack, A2DP/AAC, or
the optional WirePlumber policies managed by the user.

## Compatibility and limitations

The verified baseline is Ubuntu 22.04, BlueZ 5.64, PipeWire/pipewire-pulse
0.3.48, WirePlumber 0.4.8, FDK-AAC 2.0.2, and AirPods Pro. Rust 1.85 or newer is
required only when building from source.

The repository does not yet declare a redistribution license. Select and add a
license before publishing `.deb` artifacts publicly.

Detailed installation, verification, troubleshooting, and cleanup steps are in
[docs/INSTALLATION.md](docs/INSTALLATION.md).
