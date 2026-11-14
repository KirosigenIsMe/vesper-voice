A native voice studio written in Rust. Free forever: no PRO tier, accounts or subscriptions.

### New in 1.3.1

- **10 interface languages:** English (default), Russian, German, French, Spanish, Portuguese, Italian, Polish, Turkish and Ukrainian. Change language instantly in the sidebar or Settings; the choice is remembered.
- **A cleaner studio:** compact voice cards, a simpler transport bar, fewer repeated status labels and no decorative processing-chain buttons. Connection help and diagnostics are available on demand in Settings.
- **Presets made simpler:** save custom voices, import by file selection or drag-and-drop, and export to a location you choose. Cancelling the file picker leaves the preset unchanged.
- **Upgrade compatibility:** existing audio parameters, favorites and custom presets are preserved. Voice names stay the same across languages.
- **Settings loading is independent of audio hardware:** devices are resolved once when opening the studio, rather than during each settings deserialization.
- **Localized setup:** the installer supports the same ten languages, with English as its fallback. The license and introductory notice may remain in English.

### Downloads

**Standard installation:** download `Vesper-…-Setup-x64.exe`. Setup includes the base VB-CABLE driver, checks for an existing installation and installs it automatically if needed. Windows will request administrator privileges; the first driver installation may require a restart.

**Portable use:** extract `Vesper-…-Portable-x64.zip` and run `Vesper.exe`. Routing processed audio to other applications requires VB-CABLE. A standalone `Vesper.exe` is also available, with matching sources in the Source ZIP. Checksums are provided in `SHA256SUMS.txt`.

- 24 voices and 26 manual controls.
- Soundboard, custom WAV files and processed voice recording.
- Global F6 / F7 / F8 hotkeys.
- Windows default microphone routing with restoration on stop or crash.
- Optimized DSP and local, offline audio processing.

Technical audio/Windows error details may appear in English or the operating system's language. The UI translations are included in the executable; no language downloads or internet connection are required.

These are DSP effects, not neural voice cloning. Applications explicitly using a physical microphone need to select CABLE Output. Release binaries do not yet have a publisher code signature; SHA-256 verifies file integrity but does not replace a digital signature.

VB-CABLE is a separate donationware product by **VB-Audio Software**. Author: [vb-cable.com](https://www.vb-cable.com/). [Support VB-Audio and review licensing terms](https://vb-audio.com/Services/licensing.htm). Its original files and license are preserved unchanged; VESPER's GPL license does not cover the driver.
