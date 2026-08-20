# Installation, verification, and troubleshooting

This guide covers the binary Debian package. Building and development are
documented separately in [PACKAGING.md](PACKAGING.md) and
[DEVELOPMENT.md](DEVELOPMENT.md).

## Supported baseline

The current verified environment is Ubuntu 22.04, BlueZ 5.64,
PipeWire/pipewire-pulse 0.3.48, WirePlumber 0.4.8, and FDK-AAC 2.0.2. Other
AirPods models, firmware versions, and newer Debian/Ubuntu releases may behave
differently because AACP and the BLE battery payload are undocumented.

The package does not replace or reconfigure the existing audio stack. Before
installing, confirm that Bluetooth and the PulseAudio-compatible `pactl`
endpoint already work:

```bash
bluetoothctl show
pactl info
```

## Install

Install the package through APT, not by extracting it manually:

```bash
sudo apt install ./airpods-hires-mic_0.1.0_amd64.deb
```

APT installs the application binaries, desktop entry, icon, and the system-wide
systemd user unit. It does not create files in a user's home directory.

On GNOME, the tray requires the recommended
`gnome-shell-extension-appindicator` package. Enable **AppIndicator and
KStatusNotifierItem Support** and sign out/in before relying on the window's
close-to-tray behavior. Other desktops need StatusNotifier/AppIndicator support.

## Configure the current user

1. Pair and connect the AirPods using the desktop Bluetooth settings.
2. Open **AirPods Hi-Res Mic** from the application menu.
3. Select the connected device and press **Use this device**.
4. Adjust Gain and Limiter if needed.
5. Press **Start mic**.

The UI creates:

```text
~/.config/airpods-hires-mic/environment
```

The user service includes `ConditionPathExists`, so users who have never
selected a device do not get a failing background service.

## Verify

Check the service and logs:

```bash
systemctl --user status airpods-hires-mic.service
journalctl --user -u airpods-hires-mic.service -f
```

A working microphone stream normally logs:

```text
[bt] connected ... on PSM 0x1001
[aacp] session initialized
[aacp] hi-res microphone START sent
[aacp] first 0x58 audio SDU
[pw] virtual microphone created: AirPodsHiRes
```

Confirm and record the virtual source:

```bash
pactl list short sources
parecord --device=AirPodsHiRes --file-format=wav test.wav
paplay test.wav
```

The source appears only after the decoder receives a valid audio frame.

## Battery diagnostic

Open the case near the computer and run:

```bash
airpods-battery
```

BLE addresses are randomized and unencrypted advertisements usually provide
10% resolution. The case can legitimately report `unavailable` when its value
is absent from the current advertisement.

## Upgrade

Install the newer package over the current version:

```bash
sudo apt install ./airpods-hires-mic_NEW-VERSION_ARCH.deb
```

The package replaces system-owned files and preserves the current user's
selected device and microphone settings.

## Remove and purge

Remove the package while preserving user settings:

```bash
airpods-hires-mic-remove --keep-config
```

Purge the package and the current user's production environment:

```bash
airpods-hires-mic-remove --purge
```

The installed helper stops the current user's service before invoking APT.
When running from a source checkout, use `./uninstall.sh --package` or
`./uninstall.sh --purge` for the same lifecycle.

Debian maintainer scripts do not scan or delete arbitrary `/home/*`
directories. Other users can remove their own configuration separately.

Removal does not delete Bluetooth pairings or change PipeWire, WirePlumber,
A2DP, AAC, or an optional headset-profile policy created by the user.

## Troubleshooting

### No connected device appears on first launch

Connect the AirPods in Bluetooth settings, verify `Connected: yes`, then press
Refresh in the application:

```bash
bluetoothctl devices
bluetoothctl info AA:BB:CC:DD:EE:FF
```

### Battery scan times out

Open the AirPods case close to the Bluetooth adapter and run `airpods-battery`.
If the CLI succeeds but the UI does not, restart the desktop application so it
loads the installed backend version.

### Service starts but `AirPodsHiRes` is missing

Inspect the journal for the first `0x58` packet and decoder format line. The
AirPods must be connected and actively providing their proprietary microphone
stream before the virtual source is created.

### Audio is slow, deep, robotic, or distorted

The journal should report a 64 kHz PCM clock. The observed stream delivers 480
mono samples every 7.5 ms even though FDK reports a 48 kHz coding rate.

### An application switches playback to HFP/HSP

Select `AirPodsHiRes` explicitly as the application's microphone. This project
never calls `pactl set-card-profile` and does not install a global WirePlumber
policy. Profile-policy changes remain an explicit user choice because their
format differs across WirePlumber releases.
