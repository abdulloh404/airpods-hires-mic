# AirPods Hi-Res Mic for Linux

`airpods-hires-mic` exposes the proprietary microphone stream from AirPods as a
virtual Linux microphone while keeping Bluetooth playback on the high-quality
A2DP/AAC profile.

The program connects to the AirPods AACP L2CAP channel, receives AAC-ELD audio,
decodes it with FDK-AAC, and publishes mono PCM through PipeWire's PulseAudio
compatibility layer as:

```text
AirPods Hi-Res Mic (source name: AirPodsHiRes)
```

> [!IMPORTANT]
> This is experimental, unofficial software. It uses an undocumented AirPods
> protocol and may stop working after firmware updates. It is not affiliated
> with or endorsed by Apple.

## What it does

- Receives the AirPods proprietary high-resolution microphone stream over BlueZ.
- Decodes AAC-ELD with `libfdk-aac`.
- Creates a mono, 16-bit, 64 kHz virtual microphone named `AirPodsHiRes`.
- Runs as a per-user `systemd` service and starts automatically after login.
- Sends the AirPods microphone STOP command and removes the virtual source on
  normal shutdown.
- Does not change the active Bluetooth profile, A2DP codec, AAC settings,
  PipeWire configuration, or files under `/etc`.

## Compatibility

The current implementation is Linux-only and the installer supports
Ubuntu/Debian systems.

| Component | Project requirement | Verified environment |
| --- | --- | --- |
| Rust | `1.85` or newer (Rust 2024 edition) | `1.97.1` |
| FDK-AAC development library | `2.0` or newer, enforced at build time | `2.0.2` |
| PipeWire | `0.3.48` or newer supported baseline | `0.3.48` |
| pipewire-pulse | Same version family as PipeWire; Pulse server must be reachable | `0.3.48` |
| WirePlumber | `0.4.8` or newer supported baseline | `0.4.8` |
| BlueZ | `5.64` or newer supported baseline | `5.64` |

Only Rust and FDK-AAC have hard version checks in the build. The remaining
versions are the oldest configuration tested by this project; older versions
may work but are not supported yet.

The project has been tested by its author with AirPods Pro. Other AirPods and
Beats models have not yet been verified.

## Download

Clone the repository:

```bash
git clone https://github.com/abdulloh404/airpods-linux.git
cd airpods-linux
```

Alternatively, download the source ZIP from:

```text
https://github.com/abdulloh404/airpods-linux/archive/refs/heads/main.zip
```

Extract it, open a terminal in `airpods-linux-main`, and make the scripts
executable if necessary:

```bash
chmod +x install.sh uninstall.sh
```

## Prerequisites

1. Install Rust `1.85` or newer using [rustup](https://rustup.rs/), then open a
   new terminal and verify it:

   ```bash
   rustc --version
   cargo --version
   ```

2. Use PipeWire with its PulseAudio compatibility service and WirePlumber.
   Verify the running audio server:

   ```bash
   pactl info
   systemctl --user status pipewire.service pipewire-pulse.service wireplumber.service
   ```

   `pactl info` should report a reachable PulseAudio server, normally
   `PulseAudio (on PipeWire ...)`.

3. Pair and connect the AirPods before installing:

   ```bash
   bluetoothctl devices
   bluetoothctl connect AA:BB:CC:DD:EE:FF
   bluetoothctl info AA:BB:CC:DD:EE:FF
   ```

   Replace the example address with the AirPods MAC address. The last command
   must show `Connected: yes`.

The installer checks and, when necessary, installs these Ubuntu/Debian build
packages with `apt`:

```text
build-essential
pkg-config
libdbus-1-dev
libfdk-aac-dev (version 2.0 or newer)
bluez
pulseaudio-utils
```

It intentionally does not replace or reconfigure the machine's existing audio
stack. If PipeWire, pipewire-pulse, WirePlumber, and the BlueZ PipeWire plugin
are not already installed, install the distribution-supported packages first.

On Ubuntu, `libfdk-aac-dev` may require the `multiverse` repository:

```bash
sudo add-apt-repository multiverse
sudo apt update
```

## Install and start

Do not run the installer itself with `sudo`. Run it as the logged-in desktop
user and pass the connected AirPods MAC address:

```bash
./install.sh --device AA:BB:CC:DD:EE:FF
```

The installer will:

1. Check the OS, required commands, dependencies, MAC address, and user
   `systemd` session.
2. Build a release binary with Cargo.
3. Install the binary to `~/.local/bin/airpods-hires-mic`.
4. Save the MAC address in
   `~/.config/airpods-hires-mic/environment`.
5. Install and enable
   `~/.config/systemd/user/airpods-hires-mic.service`.
6. Start the service immediately.

Check the service:

```bash
systemctl --user status airpods-hires-mic.service
```

Follow live logs:

```bash
journalctl --user -u airpods-hires-mic.service -f
```

A healthy stream normally includes messages similar to:

```text
[bt] connected AA:BB:CC:DD:EE:FF on PSM 0x1001
[aacp] session initialized
[aacp] hi-res microphone START sent
[pw] virtual microphone created: AirPodsHiRes
[audio] packets=... dropped=0
```

The virtual microphone is created only after the first valid AirPods audio
frame arrives. It may therefore appear a moment after the service starts.

## Use the microphone

List available input sources:

```bash
pactl list short sources
```

Select **AirPods Hi-Res Mic** in GNOME Sound settings or in the application's
microphone selector. Do not select the normal Bluetooth AirPods microphone,
because that source uses HFP/HSP and may change the headset profile.

Optionally make the virtual source the default microphone:

```bash
pactl set-default-source AirPodsHiRes
```

Applications that were already open may cache the old input. Reopen their audio
settings or restart the application after selecting `AirPodsHiRes`.

Record a quick test, press `Ctrl+C` after speaking, then play it back:

```bash
parecord --device=AirPodsHiRes --file-format=wav test.wav
paplay test.wav
```

## Service commands

```bash
# Start
systemctl --user start airpods-hires-mic.service

# Stop and clean up the virtual microphone
systemctl --user stop airpods-hires-mic.service

# Restart after reconnecting the AirPods
systemctl --user restart airpods-hires-mic.service

# Enable automatic start after login
systemctl --user enable airpods-hires-mic.service

# Disable automatic start
systemctl --user disable airpods-hires-mic.service
```

The service restarts after failures with a 10-second delay. It does not connect
the AirPods automatically; BlueZ must report the configured device as connected.

## Update or change the AirPods

Pull the latest source and reinstall:

```bash
git pull
./install.sh
```

When no `--device` is supplied, the installer reuses the MAC address from the
existing environment file. To use a different pair of AirPods:

```bash
./install.sh --device 11:22:33:44:55:66
```

## Prevent Bluetooth profile auto-switching

This program never changes the Bluetooth profile. Some communication
applications can ask WirePlumber to select the real Bluetooth microphone,
which switches playback from A2DP/AAC to HFP/HSP.

First, set `AirPodsHiRes` as the default source and select it explicitly inside
the application. On the verified WirePlumber 0.4 setup, automatic headset
profile switching can also be disabled by creating:

```text
~/.config/wireplumber/policy.lua.d/90-airpods-hires-mic.lua
```

with this content:

```lua
bluetooth_policy.policy["media-role.use-headset-profile"] = false
```

Apply the change by logging out and back in, or by restarting WirePlumber:

```bash
systemctl --user restart wireplumber.service
```

Restarting WirePlumber temporarily interrupts desktop audio. This policy affects
automatic switching for all Bluetooth headsets in the user session, but manual
profile selection remains available. WirePlumber 0.5 uses a different
configuration format; do not copy this Lua fragment to a 0.5 installation.

This optional policy is not installed or removed by this project.

## Diagnostic mode

Run transport diagnostics without decoding audio or creating a virtual
microphone:

```bash
airpods-hires-mic --device AA:BB:CC:DD:EE:FF --transport-only --verbose
```

Only one instance can run at a time. Stop the user service before starting a
manual diagnostic instance.

## Uninstall

Run from the cloned source directory:

```bash
./uninstall.sh
```

The uninstaller disables and stops the user service, removes the installed
binary and project configuration, unloads the owned virtual microphone module,
and removes its FIFO and lock files.

It does not remove system packages, Rust, the cloned source directory, Bluetooth
pairings, PipeWire settings, A2DP, or AAC configuration. If you manually created
the optional WirePlumber policy above, remove that file yourself.

## Troubleshooting

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for dependency checks, detailed
verification, profile-switch guidance, common errors, and complete cleanup.

## Architecture

```text
AirPods
  -> Bluetooth BR/EDR L2CAP, AACP PSM 0x1001
  -> AACP 0x58 audio SDUs
  -> AAC-ELD access units
  -> FDK-AAC decoder
  -> mono s16le PCM, 64 kHz clock
  -> per-user FIFO in XDG_RUNTIME_DIR
  -> pactl module-pipe-source
  -> AirPodsHiRes virtual microphone
  -> browser / meeting / recording application
```

The modules are separated so that the transport, framing, decoder, virtual
microphone, and application lifecycle can later be reused by a GTK4 frontend.
