A native voice studio written in Rust. Free forever: no PRO tier, accounts or subscriptions.

**Standard installation:** download `Vesper-…-Setup-x64.exe`. Setup includes the base VB-CABLE driver, checks for an existing installation and installs it automatically if needed. Windows will request administrator privileges; the first driver installation may require a restart.

**Portable use:** extract `Vesper-…-Portable-x64.zip` and run `Vesper.exe`. Routing processed audio to other applications requires VB-CABLE. A standalone `Vesper.exe` is also available, with matching sources in the Source ZIP. Checksums are provided in `SHA256SUMS.txt`.

- 24 voices and 26 manual controls.
- Soundboard, custom WAV files and processed voice recording.
- Global F6 / F7 / F8 hotkeys.
- Windows default microphone routing with restoration on stop or crash.
- Optimized DSP and local, offline audio processing.

**Interface language:** the 1.2.0 application UI is in Russian. Repository documentation and release notes are in English.

These are DSP effects, not neural voice cloning. Applications explicitly using a physical microphone need to select CABLE Output. Release binaries do not yet have a publisher code signature; SHA-256 verifies file integrity but does not replace a digital signature.

VB-CABLE is a separate donationware product by **VB-Audio Software**. Author: [vb-cable.com](https://www.vb-cable.com/). [Support VB-Audio and review licensing terms](https://vb-audio.com/Services/licensing.htm). Its original files and license are preserved unchanged; VESPER's GPL license does not cover the driver.
