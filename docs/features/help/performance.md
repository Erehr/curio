# Performance

Curio uses one SQLite writer thread, concurrent reader connections and separate background worker pools so browsing large libraries stays responsive during imports and scans.

## Background work

Background tasks run in separate, non-blocking queues:

| Queue | What it does | Impact while running |
|---|---|---|
| **Scanning & watching** | Walks indexed Locations for new, modified, moved or missing files and watches for filesystem events. | Assets appear in the grid as Curio discovers them. |
| **Hashing** | Computes partial hashes and full BLAKE3 content hashes for move tracking and exact duplicate detection. | Runs in the background and caches results in SQLite. |
| **Thumbnails & palettes** | Decodes images, RAWs, video frames, fonts, audio waveforms and plugin thumbnails, then extracts dominant colors for Spectrum and Color filters. | Visible assets in the viewport are generated first. |
| **Metadata readers (fields)** | Runs built-in EXIF, audio, video and font readers alongside plugin extractors for enabled fields. | Fills custom fields progressively without blocking viewing. |

## Workers

- Reopening a library loads the database right away and displays cached thumbnails without waiting for a full disk scan.
- Background scans and thumbnailing resume automatically. Adjust **Thumbnail workers** and maintenance timing in **Settings › General › Background work**.
- Collapsed **Group by** sections and off-screen rows in virtualized Grid and List views skip layout and thumbnail loading until you scroll to them.

## Jobs

- Imports, location rescans, duplicate scans, grouping passes and long-running plugin tasks appear as progress toasts in the corner of the window with a **Cancel** button.
- External scripts and plugins can monitor active and recent jobs (kept for 10 minutes) through the [Jobs API](../../api/operations/job.md) or [Job events](../../api/events/jobs.md).

## Field diagnostics

If a search or filter returns fewer assets than expected for a file-reading field (such as Camera, Author, Prompt or Triangles):

1. Open **Settings › Fields** and find the field's source section.
2. Check for **waiting**, **failed** or **Not scanned · Scan now** indicators beside the section or field.
3. Confirm the field's toggle is on. Unindexed fields are read on demand when you inspect an asset rather than stored for search.
4. Click any **failed** count to inspect the affected files and error messages, then click **Try again**.
5. Click **Rescan** and choose **Missing only** or **Everything** to re-read files. Hand-edited overrides (marked with a pencil in the Inspector) stay intact.

## Cache

Thumbnails and audio waveforms are stored in the library's cache folder, with transcoded playable video copies in a separate cache:

- Open **Settings › Storage** to inspect or move the thumbnail cache and adjust its **Cache limit**.
- Click **Clear thumbnails** anytime to reclaim disk space. Curio rebuilds thumbnails as you browse without touching your files or metadata.

## Previews

- **Stale or broken thumbnail**: Right-click the asset and choose **More › Refresh thumbnail**. Choose **More › Refresh colors** if its Spectrum color position looks wrong.
- **Large pictures**: Opening a large image displays a scaled preview first and decodes higher detail as you zoom. Zoom previews stay in memory rather than on disk.
- **Video codecs**: Formats your platform's WebView cannot play natively use FFmpeg to build a cached playable copy. On Linux, playback relies on system FFmpeg and GStreamer packages. See [Video & audio](../viewer/media.md) for codec and FFmpeg details.
