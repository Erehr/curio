# Install

Curio runs locally on Windows and Linux, with ad-hoc signed macOS `.dmg` builds available. No account, cloud connection or administrator privileges are required.

## Windows

Each release includes two Windows x64 packages:

| Package | File name | Behavior |
|---|---|---|
| **Installer (NSIS)** | `Curio_<version>_x64-setup.exe` | Installs per user into `%LOCALAPPDATA%\Curio` without administrator rights. Bundles FFmpeg in `resources\ffmpeg\` and downloads Microsoft Edge WebView2 only if missing. |
| **Portable ZIP** | `Curio-<version>-windows-x64-portable.zip` | Extract anywhere and run `Curio.exe`. Ships with FFmpeg, license notices and an empty `Curio data` folder beside `Curio.exe` so portable mode is active on first launch. |

### Silent install

The NSIS installer supports unattended installation from PowerShell or Command Prompt:

```powershell
.\Curio_1.0.0_x64-setup.exe /S
# Install to a custom folder:
.\Curio_1.0.0_x64-setup.exe /S /D=D:\Apps\Curio
```

### Upgrade & uninstall

- Running a newer installer over an existing installation offers to uninstall the previous version first. Choosing **Uninstall** removes old shortcuts and taskbar pins; choosing **Do not uninstall** upgrades in place and keeps them.
- The installer refuses to downgrade over a newer installed version.
- Uninstalling Curio removes program files in `%LOCALAPPDATA%\Curio` and leaves your settings (`%APPDATA%\Curio`) and `.curio` libraries untouched.

## Linux

Each release includes three Linux x64 packages:

| Package | File name | Usage |
|---|---|---|
| **Debian / Ubuntu (`.deb`)** | `Curio_<version>_amd64.deb` | Install with `sudo apt install ./Curio_<version>_amd64.deb` (installs the `curio` binary, `.desktop` launcher, icons and WebKitGTK/GTK dependencies). |
| **Fedora / openSUSE (`.rpm`)** | `Curio-<version>-1.x86_64.rpm` | Install with `sudo dnf install ./Curio-<version>-1.x86_64.rpm`. Fedora's `ffmpeg-free` lacks some codecs (H.264, H.265); RPM Fusion's `ffmpeg` includes them. |
| **AppImage** | `Curio_<version>_amd64.AppImage` | Run across distributions without installation: `chmod +x Curio_<version>_amd64.AppImage` and launch directly. Carries GStreamer plugins for playback and uses the system `ffmpeg`. |

Unlike Windows and macOS, Linux packages do not bundle FFmpeg. Media probing, thumbnails and playback rely on packages from your distribution:

| Dependency | Role | Required |
|---|---|---|
| `libwebkit2gtk-4.1`, `libgtk-3`, `xdg-utils` | Desktop window, native dialogs, drag-out and portal integration | Yes |
| `ffmpeg` | Video/audio probing, thumbnails, waveforms and playable-copy transcoding | Yes |
| `gstreamer`, `gst-plugins-base`, `gst-plugins-good` | WebKitGTK audio and video playback (bundled in AppImage) | Yes |
| `gst-libav`, `gst-plugins-bad` | Extra codecs (H.264, HEVC, AAC, VP9, AV1) | Recommended |

!!! note "Linux video playback"
    On some Linux WebKitGTK setups, opening full video playback in the viewer can terminate the WebKit web process depending on GPU and GStreamer driver combinations. Hover previews and audio playback work across setups; see [Play video and audio](../features/viewer/media.md) for codec and transcoding details.

## macOS

Curio is primarily developed and tested on Windows and Linux. macOS `.dmg` builds are available for Apple Silicon and Intel, ad-hoc signed and not notarized with Apple. They bundle FFmpeg in `Curio.app/Contents/Resources/resources/ffmpeg/`.

Curio requires macOS 12 Monterey or later with Safari 16.4 or later installed, because the window uses the system WebKit updated through Safari. macOS 13.3 Ventura and later include it; on Monterey, install Safari updates from **System Preferences › Software Update**.

1. Open the `.dmg` image and drag **Curio.app** into `/Applications`.
2. Clear the quarantine flag once in Terminal so Gatekeeper allows the ad-hoc signed app to launch:

```bash
xattr -cr /Applications/Curio.app
```

Alternatively, launch Curio once and choose **Done** when macOS says "Curio" Not Opened, then choose **Open Anyway** for Curio in **System Settings › Privacy & Security** and confirm with your password.

## Downloads

Download installers and archives from [**GitHub Releases**](https://github.com/Erehr/curio/releases). Each release includes:

- Windows NSIS installer (`Curio_<version>_x64-setup.exe`) and portable ZIP (`Curio-<version>-windows-x64-portable.zip`)
- Linux `.deb` (`Curio_<version>_amd64.deb`), `.rpm` (`Curio-<version>-1.x86_64.rpm`) and AppImage (`Curio_<version>_amd64.AppImage`)
- macOS `.dmg` for Apple Silicon (`Curio_<version>_aarch64.dmg`) and Intel (`Curio_<version>_x64.dmg`), ad-hoc signed, not notarized
- Bundled FFmpeg build info and source archive (`Curio-<version>-ffmpeg-source.tar`)
- Companion repositories: the [**Send to Curio**](https://github.com/Erehr/curio-extension) browser extension and optional official plugins ([**curio-plugins**](https://github.com/Erehr/curio-plugins): Reader, 3D models and AI generation)

### Launch flags

Pass flags to `Curio.exe` (Windows), `curio` (Linux) or `/Applications/Curio.app/Contents/MacOS/curio` (macOS) from a terminal or shortcut:

| Flag | Effect |
|---|---|
| `--library <path>` | Open a specific `.curio` library folder immediately. |
| `--welcome` | Open the Welcome screen without reopening the last library. |
| `--portable` | Start in portable mode, storing app data in `Curio data` beside the executable. |
| `--tray` | Open the last library hidden in the system tray or menu bar (used when launching at login). |
| `--safe-mode` | Ignore custom themes and user CSS to recover from broken styling. |
| `--background` | Open windows behind existing windows without stealing focus (`CURIO_BACKGROUND=1` does the same). |

## Updates

Curio keeps application settings and library data separate from the program binary, so upgrading or reinstalling never overwrites libraries or preferences:

| Data | Windows (installed) | Linux | macOS |
|---|---|---|---|
| Settings, themes, API tokens, recent list and `curio.log` | `%APPDATA%\Curio` | `~/.config/Curio` | `~/Library/Application Support/Curio` |
| Default folder for new libraries and app-level plugins | `%APPDATA%\Curio` | `~/.local/share/Curio` | `~/Library/Application Support/Curio` |
| Library database, thumbnails, backups and managed files | Inside each `<Name>.curio` folder | Inside each `<Name>.curio` folder | Inside each `<Name>.curio` folder |

!!! tip "Automatic updates"
    Built-in signed auto-updates are planned for a future release. Until then, install the newer package over your existing installation (or replace the program files in a portable folder while keeping `Curio data/`).

## Portable mode

Portable mode stores Curio configuration, themes, tokens, plugins and recent-library paths beside the executable for USB drives or synced tool folders.

### Enable

Use either method:

1. Create an empty folder named `Curio data` in the same folder as `Curio.exe` (Windows) or the `.AppImage` / binary (Linux).
2. Start Curio once with `--portable` to create `Curio data` automatically.

### Moving

- Curio stores all settings, themes, API tokens and app-level plugins inside `Curio data/`.
- Any `.curio` library on the **same drive** as the portable executable is recorded with a **relative path**. When a USB drive receives a different drive letter on another Windows computer (for example `E:\` instead of `G:\`), Curio resolves the library without prompting you to relocate it.
- Libraries on another drive keep absolute paths and appear offline while that drive is absent.

For full details on how libraries store files and handle external drives, see [Copied files and indexed locations](../features/libraries/storage.md).

## License

Curio is free software under the **GNU General Public License, version 3 (GPL-3.0)**, covering the desktop app, `curio.js`, starter plugin templates and documentation examples. Bundled third-party components keep their own licenses:

| Component | License |
|---|---|
| Tauri, Vue, OpenVue (MIT fork of PrimeVue 4), Tabler Icons | MIT or Apache-2.0 |
| FFmpeg (Windows bundle, BtbN's GPL shared build) | GPL-3.0 (separate executable) |
| FFmpeg (macOS bundle, Martin Riedl's static GPL builds) | GPL-3.0 (separate executable) |

Each release includes `Curio-<version>-ffmpeg-source.tar` with the exact FFmpeg build URLs, SHA-256 checksums, source archives and compiled library lists. Optional plugins in `curio-plugins` carry their own third-party components (such as `pdf.js` and `foliate-js` in the Reader plugin).
