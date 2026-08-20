# AirPods Hi-Res Mic Desktop

This directory contains the optional Tauri 2 settings application. The existing
Rust audio service remains the only process that connects to the AirPods AACP
microphone; the desktop application only reads BlueZ status and controls the
fixed `airpods-hires-mic.service` user unit.

## Linux prerequisites

On Ubuntu/Debian, install the Tauri 2 WebKitGTK and AppIndicator dependencies:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```

GTK4 does not provide a system-tray API. Tauri uses AppIndicator/StatusNotifier
on Linux, and GNOME requires an AppIndicator extension for tray icons to appear.

## Build and run

```bash
cd ui
npm install
npm run tauri build
./src-tauri/target/release/airpods-hires-mic-desktop
```

Closing the settings window hides it. Use **Open Settings** in the tray menu to
show it again. Quitting the settings application does not stop the microphone
service.

Battery percentage is read from BlueZ `org.bluez.Battery1` when available.
AirPods models or BlueZ versions that do not expose this interface are shown as
`Unavailable`; the application does not invent a battery value.
