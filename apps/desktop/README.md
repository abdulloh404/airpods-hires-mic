# AirPods Hi-Res Mic Desktop

This directory contains the optional Tauri 2 settings application. The existing
Rust audio service remains the only process that connects to the AirPods AACP
microphone; the desktop application reads BlueZ status and controls the
matching user unit for its build mode.

## Linux prerequisites

On Ubuntu/Debian, install the Tauri 2 WebKitGTK and AppIndicator dependencies:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```

GTK4 does not provide a system-tray API. Tauri uses AppIndicator/StatusNotifier
on Linux, and GNOME requires the `gnome-shell-extension-appindicator` package
to be installed and enabled for tray icons to appear. Do not rely on
close-to-tray until the icon is visible.

## Build and run

```bash
cd apps/desktop
npm install
npm run tauri build
../../target/release/airpods-hires-mic-desktop
```

For development, run `npm start`. The debug desktop binary only uses
`~/.config/airpods-hires-mic/dev.environment` and controls
`airpods-hires-mic-dev.service`. A release build uses
`~/.config/airpods-hires-mic/environment` and
`airpods-hires-mic.service`, so it cannot restart the production microphone
service while developing the UI.

Closing the settings window hides it. Use **Open Settings** in the tray menu to
show it again. Quitting the settings application does not stop the microphone
service.

Battery data is read from a short BLE scan and shown separately for left and
right earbuds and the case when advertised. Some AirPods advertisements do not
include every value, which is shown as unavailable rather than guessed.
