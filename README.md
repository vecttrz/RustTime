# RustTime

[![License: MIT](https://img.shields.io/badge/license-MIT-yellow.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-1.100%2B-orange)
![Windows](https://img.shields.io/badge/Windows-10%2F11-blue)
![Version](https://img.shields.io/badge/version-0.1.0-green)

A Windows desktop application written in Rust that lets you schedule a **shutdown** or **restart**, cancel or disable that schedule, and inspect **disk** and **battery** information.

![RustTime in English](docs/Screenshot.png)

📄 **Product description and installation instructions:** [docs/PRODUCT.md](docs/PRODUCT.md)

> If the repository is hosted under a different user/name, update the download links in this section.

## Downloads

These files are published on the [releases page](https://github.com/vecttrz/RustTime/releases):

| Artifact | File | Recommended use |
| --- | --- | --- |
| **Installer** | `RustTime-v0.1.0-setup.exe` | Installs to `{Program Files}\RustTime`, with a shortcut and uninstaller. |
| **Portable** | `RustTime-v0.1.0-portable.zip` | Run without installing, for example from a USB drive. |
| **Checksums** | `RustTime-v0.1.0-checksums.txt` | SHA-256 verification of the files above. |

- [Download installer](https://github.com/vecttrz/RustTime/releases/download/v0.1.0/RustTime-v0.1.0-setup.exe)
- [Download portable version](https://github.com/vecttrz/RustTime/releases/download/v0.1.0/RustTime-v0.1.0-portable.zip)
- [View checksums](https://github.com/vecttrz/RustTime/releases/download/v0.1.0/RustTime-v0.1.0-checksums.txt)

To verify a file in PowerShell:

```powershell
Get-FileHash .\RustTime-v0.1.0-setup.exe -Algorithm SHA256
```

## Features

### Shutdown / Restart

- Choose **hours** and **minutes** (up to 365 days).
- Choose whether to **shut down** or **restart** the computer.
- **Schedule** button: runs `shutdown /s /t <seconds>` or `shutdown /r /t <seconds>`. The Windows countdown keeps running even if RustTime is closed.
- **Disable automatic shutdown** checkbox: runs `shutdown /a` and blocks new schedules while it is checked.
- **Cancel current operation** button to abort the pending schedule.
- Displays the remaining time and the result of each operation.

### Hard drives

- Shows every connected physical disk with its type, capacity, and **total power-on hours** (S.M.A.R.T., `Power_On_Hours`).
- Reading S.M.A.R.T. hours requires running the application **as administrator**; if unavailable, a button is offered to restart it with elevated permissions.

### Battery

- **Consumed cycles** (from the `powercfg /batteryreport` XML report).
- **Current charge** (%) and battery **health** (current capacity versus design capacity).

### Interface

- **Two-column horizontal layout**: shutdown controls on the **left** and disk/battery information on the **right**.
- All information is **centered** within its column and grouped into sections, with **side margins** in the form.
- The window **cannot be maximized or resized manually**.
- The **height adjusts automatically to the content** (for example, it grows when the countdown appears or more disks are detected).
- **Automatic language:** if Windows is set to Spanish the interface is shown in Spanish; any other language shows English.
- The icon from `design/Icon.ico` is embedded in the executable and is also used as the window icon.
- System data refreshes on startup, when **Refresh** is clicked, and automatically every 5 minutes.
- Development credits are visible at the bottom of the window.

## Installation

### Installer

1. Download `RustTime-v0.1.0-setup.exe`.
2. Run it and follow the wizard.
3. Open RustTime from the Start menu or the desktop shortcut.

The uninstaller is located in the installation folder and in *Add or remove programs*.

### Portable

1. Download `RustTime-v0.1.0-portable.zip`.
2. Extract `RustTime.exe`.
3. Run it directly; no installation or registry writes are required.

## Requirements

- Windows 10 or 11.
- Administrator permissions for disk S.M.A.R.T. hours (the application can restart itself elevated).

## Building from source

You need [Rust](https://rustup.rs/) 1.100 or newer and, to generate the installer, [Inno Setup 6](https://jrsoftware.org/isinfo.php).

```powershell
# Build the application
cargo build --release

# Create the installer and portable ZIP in the release\ folder
powershell -ExecutionPolicy Bypass -File .\scripts\build-release.ps1
```

The compiled executable is written to `target\release\rusttime.exe` and the release artifacts to `release\`.

## Quick start

1. Choose **hours** and **minutes**.
2. Select **Shut down** or **Restart**.
3. Click **Schedule**.
4. To abort, click **Cancel current operation** or check **Disable automatic shutdown**.

You can also cancel from a terminal:

```powershell
shutdown /a
```

## Troubleshooting

- **"Hours unavailable" for disks:** Windows does not expose S.M.A.R.T. counters without elevation. Click **Restart as administrator** and accept the UAC prompt.
- **The shutdown is not cancelled:** check that no other schedule is active and run `shutdown /a` as administrator.
- **Wrong language:** RustTime follows the Windows *user interface language* (`GetUserDefaultUILanguage`), not the regional format.

## Repository structure

```text
design/       Project icons (Icon.ico, Icon.png, Icon.af)
docs/         PRODUCT.md, RELEASE_NOTES.md, and Screenshot.png
installer/    Inno Setup script for the installer
scripts/      Script that builds the release artifacts
src/          Rust source code (main.rs, sysinfo.rs, i18n.rs)
build.rs      Embeds Icon.ico as an executable resource
release/      Generated artifacts (not versioned)
```

## Credits

Development: **vecttrz** · 2026

## License

Distributed under the [MIT](LICENSE) license.

> **Note:** do not run the application with a very short delay if you do not want the computer to shut down or restart.
