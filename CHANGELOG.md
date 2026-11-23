# Changelog

All notable changes to RustTime are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-23

### Added
- Schedule a Windows shutdown or restart by hours and minutes.
- Cancel the pending operation and a "disable automatic shutdown" mode.
- Countdown and status for every operation.
- Computer information: manufacturer, model, and BIOS/motherboard date.
- Physical disk information: type, capacity, and total power-on hours (S.M.A.R.T.).
- Battery information: consumed cycles, current charge, and health.
- Button to restart the application as administrator when S.M.A.R.T. is unavailable.
- Two-column horizontal, centered interface with dynamic height and a non-maximizable window.
- Automatic language detection: Spanish when Windows is set to Spanish, English otherwise.
- Application icon embedded in the executable and the window.
- Development credits: vecttrz · 2026.
- Release artifacts: installer (Inno Setup) and portable version (ZIP).

[0.1.0]: https://github.com/vecttrz/RustTime/releases/tag/v0.1.0
