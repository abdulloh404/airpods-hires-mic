# Installation, Verification, and Troubleshooting

This guide covers a source installation of `airpods-hires-mic` on Ubuntu or
Debian. The short installation path is in the project [README](../README.md).

## Support boundary

The current release is an experimental Linux implementation for AirPods. It has
been verified with AirPods Pro on this software stack:

```text
Ubuntu 22.04
PipeWire 0.3.48
pipewire-pulse 0.3.48
WirePlumber 0.4.8
BlueZ 5.64
FDK-AAC 2.0.2
Rust 1.97.1
```

The project supports PipeWire `0.3.48` and WirePlumber `0.4.8` as its current
baseline. Newer versions are expected to work for normal microphone operation,
but WirePlumber profile-policy configuration differs between the 0.4 and 0.5
series.

`build.rs` requires FDK-AAC `2.0` or newer. `Cargo.toml` uses Rust edition 2024,
which requires Rust `1.85` or newer.

## 1. Confirm the audio stack

Check the package versions:

```bash
dpkg-query -W pipewire pipewire-pulse wireplumber bluez libfdk-aac-dev
```

Check the running user services:

```bash
systemctl --user --no-pager --full status \
  pipewire.service pipewire-pulse.service wireplumber.service
```

Check the PulseAudio compatibility endpoint:

```bash
pactl info
```

The server must be reachable. On PipeWire, the server name normally contains
`PulseAudio (on PipeWire ...)`.

If the audio stack is missing, install the packages recommended by the Ubuntu
or Debian release in use. A typical PipeWire installation needs PipeWire,
pipewire-pulse, WirePlumber, and the distribution's PipeWire Bluetooth SPA
plugin. Replacing a working PulseAudio installation is outside the scope of
this project; follow the distribution's migration instructions.

## 2. Install Rust

Ubuntu 22.04's original `rustc` package is too old for Rust edition 2024. Install
a current stable toolchain with [rustup](https://rustup.rs/), then verify:

```bash
rustup default stable
rustc --version
cargo --version
```

`rustc --version` must report `1.85.0` or newer.

## 3. Install build dependencies

The installer can install these automatically. To install them manually:

```bash
sudo apt update
sudo apt install \
  build-essential \
  pkg-config \
  libdbus-1-dev \
  libfdk-aac-dev \
  bluez \
  pulseaudio-utils
```

Confirm FDK-AAC:

```bash
pkg-config --modversion fdk-aac
```

The result must be `2.0` or newer.

If Ubuntu cannot find `libfdk-aac-dev`, enable `multiverse` and refresh the
package index:

```bash
sudo add-apt-repository multiverse
sudo apt update
```

## 4. Download the source

Using Git:

```bash
git clone https://github.com/abdulloh404/airpods-linux.git
cd airpods-linux
```

Using the main-branch ZIP:

```text
https://github.com/abdulloh404/airpods-linux/archive/refs/heads/main.zip
```

After extracting the ZIP:

```bash
cd airpods-linux-main
chmod +x install.sh uninstall.sh
```

## 5. Pair and connect the AirPods

The program does not pair or connect Bluetooth devices. Pair the AirPods using
GNOME Settings or `bluetoothctl`, then find their address:

```bash
bluetoothctl devices
```

Connect and verify them:

```bash
bluetoothctl connect AA:BB:CC:DD:EE:FF
bluetoothctl info AA:BB:CC:DD:EE:FF
```

Continue only when the result includes:

```text
Connected: yes
```

The AirPods playback profile should remain A2DP. The application only logs the
active profile and never calls `pactl set-card-profile`.

## 6. Install and start the service

Run the installer as the desktop user, not as root:

```bash
./install.sh --device AA:BB:CC:DD:EE:FF
```

`sudo` is used internally only when missing Debian packages need to be
installed. User files are installed under the current user's home directory.

Installed files:

| File | Purpose |
| --- | --- |
| `~/.local/bin/airpods-hires-mic` | Release binary |
| `~/.config/airpods-hires-mic/environment` | AirPods MAC address |
| `~/.config/systemd/user/airpods-hires-mic.service` | User service |

Runtime files are created only while needed:

| File | Purpose |
| --- | --- |
| `$XDG_RUNTIME_DIR/airpods-hires-mic.lock` | Single-instance lock |
| `$XDG_RUNTIME_DIR/airpods-hires-mic.fifo` | Decoded PCM FIFO |

The installer enables and starts the service. It refuses to overwrite unsafe
symlinks or non-regular configuration paths.

## 7. Verify every stage

### Service stage

```bash
systemctl --user status airpods-hires-mic.service
```

Expected state:

```text
Active: active (running)
```

### Bluetooth and AACP stage

```bash
journalctl --user -u airpods-hires-mic.service -f
```

Expected messages:

```text
[bt] connected ... on PSM 0x1001
[aacp] session initialized
[aacp] hi-res microphone START sent
[aacp] first 0x58 audio SDU: ... bytes
```

### Decoder stage

Expected format message:

```text
[audio] format: 1 channel(s) / 64000 Hz PCM clock / 48000 Hz FDK coding rate / ...
```

The 64 kHz PCM clock is intentional. AirPods deliver 480 mono samples every
7.5 ms; treating the output as 48 kHz makes playback slow and distorted.

### Virtual microphone stage

Expected message:

```text
[pw] virtual microphone created: AirPodsHiRes
```

Confirm the source:

```bash
pactl list short sources
```

The list should include `AirPodsHiRes`.

### Recording stage

```bash
parecord --device=AirPodsHiRes --file-format=wav test.wav
```

Speak for several seconds and press `Ctrl+C`, then play the recording:

```bash
paplay test.wav
```

## 8. Select the correct input in applications

Choose **AirPods Hi-Res Mic** or `AirPodsHiRes`, not the normal Bluetooth
AirPods microphone.

Set it as the default source if desired:

```bash
pactl set-default-source AirPodsHiRes
```

Browsers and meeting applications may cache the source selected when they were
opened. Reopen the application's audio settings or restart the application.

## 9. Stop automatic HFP/HSP profile switching

WirePlumber 0.4 can automatically switch a Bluetooth device from A2DP to a
headset profile when Chrome, Firefox, Zoom, or another communication application
opens an input stream.

On WirePlumber 0.4 only, create:

```text
~/.config/wireplumber/policy.lua.d/90-airpods-hires-mic.lua
```

with:

```lua
bluetooth_policy.policy["media-role.use-headset-profile"] = false
```

Apply it by logging out and back in, or restart WirePlumber:

```bash
systemctl --user restart wireplumber.service
```

This disables automatic Bluetooth headset-profile switching for the whole user
session. It does not remove HFP/HSP profiles, so manual profile selection still
works. It also does not force an AAC codec; codec negotiation remains managed by
PipeWire and BlueZ.

WirePlumber 0.5 does not load 0.4 Lua configuration fragments. Use the official
WirePlumber documentation for the installed 0.5 release rather than copying the
fragment above.

The optional policy is a manual system preference. `install.sh` and
`uninstall.sh` deliberately do not modify it.

## 10. Updating

For a Git checkout:

```bash
git pull
./install.sh
```

The existing MAC address is reused. Supplying `--device` replaces it:

```bash
./install.sh --device 11:22:33:44:55:66
```

The installer builds the new release, stops the existing project service,
replaces its binary and service file, reloads the user service manager, and
starts the service again.

## 11. Uninstall completely

From the source checkout:

```bash
./uninstall.sh
```

The script removes all resources owned by the installed application:

- user service and its failed state;
- installed binary;
- saved AirPods address;
- `AirPodsHiRes` module owned by the project;
- runtime FIFO and single-instance lock.

It intentionally preserves:

- system packages installed through `apt`;
- PipeWire, WirePlumber, BlueZ, A2DP, and AAC configuration;
- Bluetooth pairings;
- the source checkout and Cargo build directory;
- any optional WirePlumber policy created manually.

Remove the optional WirePlumber 0.4 policy manually if it was created:

```text
~/.config/wireplumber/policy.lua.d/90-airpods-hires-mic.lua
```

Then log out and back in or restart WirePlumber.

## Common errors

### `Rust is missing`

Install a stable Rust toolchain with rustup, open a new terminal, and confirm
that both `rustc` and `cargo` are in `PATH`.

### `libfdk-aac development files are required`

Install `libfdk-aac-dev`, confirm that `pkg-config --modversion fdk-aac`
returns at least `2.0`, and rebuild.

### `PipeWire/Pulse server is unreachable`

Run `pactl info`. The installer and service must run inside the logged-in
desktop user's session, where `XDG_RUNTIME_DIR` and the user audio server are
available. Do not run the program or installer with `sudo`.

### `AirPods ... are not connected through BlueZ`

Reconnect the configured address and restart the service:

```bash
bluetoothctl connect AA:BB:CC:DD:EE:FF
systemctl --user restart airpods-hires-mic.service
```

### `AACP handshake failed` or `Transport endpoint is not connected`

Confirm the AirPods are connected, wait a few seconds after connection, and
restart the service. If it repeats, capture verbose logs with transport-only
mode after stopping the service.

### `no AACP 0x58 audio packet received within 3 seconds`

The control connection succeeded but no proprietary microphone packets
arrived. Put the AirPods in use, confirm both earbuds are active, reconnect them,
and retry. Support can vary by AirPods model and firmware.

### The service runs but `AirPodsHiRes` is missing

The source is created only after the decoder receives a valid audio frame.
Inspect the journal for the first `0x58` packet, the decoder format line, and a
virtual microphone creation line.

### Audio is slow, deep, robotic, or distorted

Confirm the journal reports a `64000 Hz PCM clock`. If a different build creates
the virtual source at 48 kHz, update and reinstall this project. Also confirm the
application has selected `AirPodsHiRes`, not a Bluetooth HFP/HSP input.

### An application changes A2DP/AAC to HFP/HSP

Select `AirPodsHiRes` in that application's microphone settings and set it as
the default source. WirePlumber 0.4 users can apply the optional auto-switch
policy described above.

### `another airpods-hires-mic instance is already running`

Do not run a manual instance while the user service is active. Use the service
or stop it before manual diagnostics.

## Manual diagnostics

The transport-only mode verifies AACP packets without requiring `pactl`, loading
a virtual microphone, or decoding AAC-ELD:

```bash
systemctl --user stop airpods-hires-mic.service
airpods-hires-mic \
  --device AA:BB:CC:DD:EE:FF \
  --transport-only \
  --verbose
```

Press `Ctrl+C` to stop. The program attempts to send the AACP STOP command on
both `SIGINT` and `SIGTERM` before exiting.

Start the service again when finished:

```bash
systemctl --user start airpods-hires-mic.service
```
