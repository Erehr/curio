# Video & audio

Curio plays video and audio through the WebView's hardware-accelerated media engine and uses FFmpeg for thumbnails, waveforms, media metadata and cached playable copies. Both players share a transport bar with persistent volume, mute, speed and end-of-playback settings.

## Playback

Open a video or audio asset in the main stage or a separate window. Click the video frame or audio cover to play or pause, or use the bottom transport bar:

- **Play, pause and step**: Play/pause, **Previous item** and **Next item** buttons (skipping missing or offline assets).
- **Volume and mute**: Click the speaker button to toggle mute, or hover it to adjust volume (`0–100%`). Volume and mute persist across assets and hover playback.
- **Timeline and scrubbing**: Click or drag anywhere on the seek bar to scrub. Hovering over a video timeline displays a frame preview above the timestamp.
- **A–B loop handles**: On videos, drag the start and end trim handles on the timeline to loop a segment continuously.
- **Playback speed**: Choose `0.25x` to `5x` for video (`0.25x`, `0.5x`, `0.75x`, `1x`, `1.25x`, `1.5x`, `2x`, `3x`, `5x`) and `0.5x` to `2x` for audio.
- **End behavior**: Cycle the repeat button between **Repeat** (default), **Play next** and **Stop**. Advancing to another video starts playback even when **Autoplay** is off.
- **Fullscreen**: Click the fullscreen button or press ++f11++. In fullscreen video, the transport bar hides and reappears when the pointer moves within 110 px of the bottom edge.
- **Orientation and frame actions**: Rotate videos in 90° steps (++r++) and mirror horizontally (++h++) on screen without changing the file. Right-clicking a video offers **Set frame as thumbnail**, **Copy this frame** (++alt+c++), **Save frame as image** (++alt+s++, hidden on read-only locations) and **Load subtitles**.

## Behavior

Configure playback options under **Settings › Preview › Video & audio playback**:

| Setting | Behavior |
|---|---|
| **Mouse wheel** | Sets whether scrolling over media **Seeks** (±5 s), adjusts **Volume** (±10%) or moves to **Previous / next item**. If **Settings › Controls › Mouse wheel in the viewer** is set to **Previous / next item**, plain scrolling navigates assets first. |
| **Play on hover** | Hovering over a video card starts muted preview playback after 300 ms; moving horizontally scrubs the clip. Hovering over an audio card shows its waveform after 250 ms, plays audio and lets you click to seek. Card mute and hover-play toggles persist app-wide. In List view, hovering a row's picture plays it the same way, without the buttons, once **Size** is at its middle or above. |
| **Stretch to fill** | Scales videos smaller than the stage up to fill the viewer while keeping their aspect ratio. |
| **Autoplay** | Starts video playback as soon as a video opens (enabled by default). |
| **Resume playback** | Saves your playback position for the last 200 assets when you pause, step away or close the viewer. Positions within the last 3 seconds clear the saved resume point. |

## Subtitles

The video player supports `.srt` (SubRip) and `.vtt` (WebVTT) subtitles:

- **Automatic sibling loading**: Any `.srt` or `.vtt` file beside the video with a matching filename loads when the video opens, with the first track active. SubRip files convert on the fly to WebVTT while preserving italic, bold and underline formatting.
- **Switch tracks**: When subtitle tracks are loaded, click **CC** on the transport bar to switch tracks or turn captions off.
- **Load an external file**: Right-click the video and choose **Load subtitles** to pick a `.srt` or `.vtt` file. Curio loads it and copies it beside the video unless the location is read-only or a matching file already exists.

## Audio

When you open an audio file (`mp3`, `wav`, `flac`, `ogg`, `oga`, `opus`, `aac`, `m4a`, `aiff`, `aif`, `wma`):

- **Cover art**: Curio shows embedded artwork stored in the file (with FFmpeg as fallback), or a sibling image named `cover`, `folder`, `front` or `albumart`. Otherwise it displays a music icon.
- **Waveform timeline**: FFmpeg computes and caches a peak waveform once per file. Both the player transport bar and grid hover previews use this waveform as a seek bar.
- **Audio tags and media info**: Curio's built-in audio tag reader extracts `artist`, `album`, `album_artist`, `title`, `track`, `disc`, `year`, `genre` and `composer` into searchable fields. FFmpeg supplies duration, codec, sample rate, bit rate and channels for the Inspector's **Audio** and **Video** widgets.

## FFmpeg & cache

The viewer plays media directly whenever the WebView supports the container and codecs, including `mp4`, `mov`, `m4v`, `webm`, `ogg`, `mp3`, `flac`, `wav` and `aac` with H.264, VP8, VP9, AV1 or platform-supported H.265.

When an unsupported container or codec is opened or hover-played (such as `mkv`, `avi`, `wmv`, `flv`, `ts`, `mpg`, `3gp` or `ogv`), FFmpeg creates a cached MP4 copy once:

- If streams are already WebView-compatible (such as H.264 inside `.mkv`), FFmpeg remuxes them with no quality loss.
- Otherwise, FFmpeg transcodes to H.264 and AAC using hardware acceleration when available, falling back to `libx264`.

Curio searches for FFmpeg in this order:

1. Custom path set for `ffmpeg` in **Settings › Plugins › Tools**.
2. `ffmpeg` and `ffprobe` in Curio's FFmpeg plugin folder (`%APPDATA%\Curio\plugins\ffmpeg` on Windows, `~/Library/Application Support/Curio/plugins/ffmpeg` on macOS, `~/.local/share/Curio/plugins/ffmpeg` on Linux) or the `CURIO_FFMPEG_DIR` environment variable.
3. The bundled FFmpeg build (Windows builds ship BtbN's GPL shared build in `resources/ffmpeg/`; macOS builds ship Martin Riedl's static GPL build in `Contents/Resources/resources/ffmpeg/`).
4. Your system `PATH`.
5. Package-manager install directories (WinGet, Scoop and Chocolatey on Windows; Homebrew, `/usr/bin` and `~/.local/bin` on macOS and Linux).

Verify which FFmpeg binary Curio uses on the **Video** entry in **Settings › Plugins**.

## Codecs

- **Windows**: Curio bundles `ffmpeg.exe` and `ffprobe.exe` in `resources/ffmpeg/`, and WebView2 handles native hardware decoding. No extra packages are required.
- **macOS**: Ad-hoc signed macOS builds (macOS 12 Monterey or later, Apple Silicon and Intel) bundle `ffmpeg` and `ffprobe`, and WebKit handles native playback.
- **Linux**: Curio uses the system `ffmpeg` package for thumbnails, metadata, waveforms and playable copies, and WebKitGTK uses GStreamer for playback. If a video fails to play, install your distribution's GStreamer plugins (`gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly` and `gst-libav` or hardware VA-API packages).
- **Without FFmpeg**: Media files still index and play directly when the WebView supports their codecs, but thumbnails, durations, waveforms and cached playable copies cannot be generated. Those assets stay queued and generate thumbnails and durations automatically the next time you open the library with FFmpeg present.
