# Development workflow

Development is isolated from a package installation. Debug builds use:

```text
~/.config/airpods-hires-mic/dev.environment
~/.config/systemd/user/airpods-hires-mic-dev.service
```

Release builds continue to use `environment` and
`airpods-hires-mic.service`. The desktop backend selects these names at compile
time, so a debug UI cannot restart the production microphone service.

## Prerequisites

Install a Rust 1.85+ toolchain, Node.js 22.12+/npm, FDK-AAC development files,
D-Bus development files, and the Tauri 2 Linux development dependencies. On Ubuntu:

```bash
sudo apt install build-essential pkg-config libdbus-1-dev libfdk-aac-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```

Install the frontend packages once:

```bash
cd apps/desktop
npm install
cd ../..
```

## First setup

Connect the AirPods and create the isolated development unit:

```bash
./scripts/dev-setup.sh --device AA:BB:CC:DD:EE:FF
```

The unit is not enabled at login. Its executable points to the workspace debug
binary at `target/debug/airpods-hires-mic`.

## Daily development

Build the daemon, restart the dev unit, and launch Tauri/Vite:

```bash
./scripts/dev.sh
```

The script refuses to start when the production service is active because both
daemons expose the same virtual microphone. It reports the conflict without
stopping the production service for you.

React changes hot-reload. After changing the daemon, core, decoder, DSP, or BLE
scanner, rebuild and restart only the dev service:

```bash
./scripts/dev-restart.sh
```

Test the battery scanner without the UI:

```bash
cargo run --package airpods-daemon --bin airpods-battery
```

## Stop and clean up

```bash
./scripts/dev-stop.sh
./scripts/dev-clean.sh
```

Cleanup removes only the development unit and `dev.environment`. It does not
touch the production package, production settings, Bluetooth profiles, A2DP,
AAC, or PipeWire configuration.
