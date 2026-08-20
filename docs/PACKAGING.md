# Debian packaging

The Debian package contains the desktop application, audio daemon, battery CLI,
desktop entry, icon, and systemd user unit. End users do not need Rust, Node.js,
or source build dependencies.

## Build environment

Build on the oldest distribution release that the resulting package must
support. Ubuntu 22.04 or Debian 12 are the intended initial runtime baselines.
Install the packages declared in `debian/control`, then put Rust/Cargo 1.85+
(for example through rustup) and Node.js 22.12+ on `PATH`. The packaging script
checks the effective tool versions because those distributions' repository
versions can be older than the workspace requires.

Before a public release, replace the `not-specified` entry in
`debian/copyright` with the project's actual redistribution license.

## Build

The Debian rules file is the single build entry point. It builds both daemon
binaries and then runs the Tauri frontend/desktop build without creating a
second Tauri package:

```bash
./scripts/build-deb.sh
```

The resulting file is written beside the repository, for example:

```text
../airpods-hires-mic_0.1.0_amd64.deb
```

## Inspect

```bash
./scripts/verify-deb.sh ../airpods-hires-mic_0.1.0_amd64.deb
```

The verifier displays package metadata and installed paths, then runs Lintian
when it is installed. Before release, also install the package in a clean VM and
verify the application menu, tray, first-run device selection, user service,
BLE battery scan, virtual microphone, upgrade, remove, and purge flows.

## Installed paths

```text
/usr/bin/airpods-hires-mic
/usr/bin/airpods-battery
/usr/bin/airpods-hires-mic-desktop
/usr/bin/airpods-hires-mic-remove
/usr/lib/systemd/user/airpods-hires-mic.service
/usr/share/applications/airpods-hires-mic.desktop
/usr/share/icons/hicolor/scalable/apps/airpods-hires-mic.svg
/usr/share/doc/airpods-hires-mic/
```

User configuration is created by the desktop application and is not owned by
the package:

```text
~/.config/airpods-hires-mic/environment
```

This keeps upgrades from overwriting the selected AirPods or microphone
settings. Package removal preserves it. The explicit
`airpods-hires-mic-remove --purge` helper removes it only for the user running
the helper.

Debian maintainer scripts stop the unit in every running systemd user manager
before removal or upgrade, then restart eligible user instances after an
upgrade. The installed removal helper also stops the invoking user's unit
before calling APT.
