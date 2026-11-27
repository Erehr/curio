# Metadata

Curio ships with six built-in metadata readers covering photos, fonts, audio tags, audio encoding, video encoding and web shortcuts. Plugin extractors run alongside them in the background metadata queue. Extraction is read-only: Curio never modifies your source files while reading metadata.

## Camera & EXIF

The built-in EXIF reader runs on `png`, `jpg`, `jpeg`, `jfif`, `tif`, `tiff`, `webp`, `heic`, `heif`, `avif`, `dng`, `nef`, `cr2` and `arw` files. It reads EXIF metadata from WebP files as well, including files exported by Pillow or ComfyUI. Text values that look like embedded ComfyUI JSON graphs (`prompt:{...}` or `workflow:{...}` stored inside `Make` or `Model`) are ignored so they are not mistaken for camera models.

| Field key | Label | Type | Notes |
|---|---|---|---|
| `camera` | **Camera** | Text | Combines EXIF `Make` and `Model` without duplicating the manufacturer prefix |
| `lens` | **Lens** | Text | EXIF `LensModel` |
| `iso` | **ISO** | Whole number | EXIF `PhotographicSensitivity` |
| `aperture` | **Aperture** | Decimal number | EXIF `FNumber`, rounded to 1 decimal place |
| `shutter` | **Shutter** | Text | EXIF `ExposureTime` formatted display value (such as `1/250`) |
| `focal_length` | **Focal length** | Decimal number | EXIF `FocalLength` in mm, rounded to 1 decimal place |
| `taken_at` | **Date taken** | Date and time | EXIF `DateTimeOriginal` normalized to `YYYY-MM-DD HH:MM:SS` |

In the default Inspector layout, these fields appear inside the **Camera** widget, which starts collapsed.

## GPS

The EXIF reader extracts geographic coordinates whenever valid GPS tags are present:

| Field key | Label | Type | Notes |
|---|---|---|---|
| `gps_lat` | **Latitude** | Decimal number | Signed degrees (`S` negative), rounded to 6 decimal places |
| `gps_lon` | **Longitude** | Decimal number | Signed degrees (`W` negative), rounded to 6 decimal places |
| `gps_alt` | **Altitude** | Decimal number | Meters above sea level (`GPSAltitudeRef = 1` negative), rounded to 1 decimal place |

Files with no GPS tags, `0, 0` placeholder coordinates or out-of-range values (`|lat| > 90` or `|lon| > 180`) leave `gps_lat`, `gps_lon` and `gps_alt` empty. Search by bounding box or presence using the [Query language](../../api/query/syntax.md):

```text
gps_lat >= 48.8 AND gps_lat <= 48.9 AND gps_lon >= 2.25 AND gps_lon <= 2.45
gps_lat:*
-gps_lat:*
```

## Rights & tags

Two authorship fields are extracted from EXIF primary IFDs:

- `artist` (**Artist**): Read from EXIF `Artist` on images and shared with the audio tag reader (`artist` on audio files).
- `copyright` (**Copyright**): Read from EXIF `Copyright`.

Curio's built-in EXIF reader does **not** convert embedded IPTC keywords, XMP subjects or Windows photo keywords into Curio tags. Embedded file metadata stays in typed fields, while Curio tags stay an explicit library taxonomy under your control. Plugins that need extra EXIF tags beyond the built-in 12 fields can extract them into their own fields or tags.

## Audio & video

Three built-in readers extract audio and video metadata:

- **Audio tags**: Reads embedded tags across `mp3`, `wav`, `flac`, `ogg`, `oga`, `opus`, `aac`, `m4a`, `aiff`, `aif` and `wma`, regardless of which media engine is enabled:
    - Extracts `title` (**Title**), `artist` (**Artist**), `album` (**Album**), `album_artist` (**Album artist**), `composer` (**Composer**), `track` (**Track**), `disc` (**Disc**), `year` (**Year**) and `genre` (**Genre**, multi-value; splits `/` and `;` separators and strips numeric ID3 genre codes such as `(17)`).
    - Uses embedded cover art (front cover first, then any embedded picture, or `cover`, `folder`, `front` or `albumart` images beside the file) as the asset's thumbnail.
- **Audio encoding**: Reads audio container properties, including distinguishing AAC and ALAC inside `.m4a`:
    - Extracts `audio_codec` (**Audio codec**), `bit_rate` (**Bit rate (kb/s)**), `channels` (**Channels**) and `sample_rate` (**Sample rate (Hz)**).
- **Video encoding**: Probes `mp4`, `mov`, `m4v`, `webm`, `mkv`, `avi`, `wmv`, `mpg`, `mpeg`, `mts`, `m2ts`, `ts`, `flv`, `f4v`, `3gp`, `ogv` and `ogm` using `ffprobe` when the FFmpeg media engine is active:
    - Extracts `video_codec` (**Video codec**), `audio_codec` (**Audio codec**), `frame_rate` (**Frame rate**, rounded to 3 decimal places), `bit_rate` (**Bit rate (kb/s)**, computed from file size and duration), `channels` (**Channels**) and `sample_rate` (**Sample rate (Hz)**).
    - Normalizes codec names across readers (`avc`, `avc1` and `h264` become `H.264`; `hevc`, `hvc1` and `h265` become `H.265`; `mp4a` becomes `AAC`) so filters list each codec once.

!!! note "When FFmpeg is not active"
    If the FFmpeg media engine is off (for example when using Mediabunny alone), the video encoding reader skips video files instead of marking them as failed. Curio probes them automatically if you enable FFmpeg later.

## Fonts

The built-in font reader inspects the first face of `ttf`, `otf`, `ttc`, `otc`, `woff`, `woff2` and `dfont` files. To keep indexes lean, Curio enables the most filter-useful font fields by default and leaves the rest off until you enable them in **Settings › Fields**:

| Field key | Label | Type | Indexed by default | Notes |
|---|---|---|---|---|
| `font_style` | **Font style** | Text | On | Style within the family (such as `Bold Italic`) |
| `font_weight` | **Font weight** | Whole number (List filter) | On | `100` (thin) to `900` (black); `400` regular, `700` bold |
| `font_width` | **Font width** | Single choice | On | 9 options from `Ultra condensed` to `Ultra expanded` |
| `font_italic` | **Italic** | Yes / No | On | Whether the face is italic |
| `font_class` | **Font category** | Single choice | On | `Sans serif`, `Serif`, `Slab serif`, `Monospace`, `Handwriting`, `Display` or `Symbol` (inferred from font tables, name clues and glyph proportions) |
| `font_scripts` | **Scripts** | Multiple choice | On | Writing systems supported by the font's glyphs |
| `font_axes` | **Variable axes** | Multiple choice | On | Variable font axes such as weight or width |
| `designer` | **Designer** | Text | On | Designer name table entry |
| `font_license` | **Font license** | Text | On | Short SPDX-style name (such as `OFL-1.1`) or first sentence of the license entry |
| `font_family` | **Font family** | Text | Off | Family name (enable in **Settings › Fields** to filter or sort by family) |
| `glyphs` | **Glyphs** | Whole number | Off | Total glyph count |
| `font_faces` | **Faces** | Whole number | Off | Number of faces inside a `.ttc`, `.otc` or `.dfont` collection |
| `foundry` | **Foundry** | Text | Off | Manufacturer or foundry name |
| `font_version` | **Font version** | Text | Off | Font version string |

See [Preview fonts](../viewer/fonts.md) for the interactive font viewer, specimen controls and **Font sample** column.

## Links & plugins

- **Web shortcuts**: Reads `.url` (Windows Internet Shortcut) and `.webloc` (macOS Property List) files and fills the asset's **URL** automatically when it is empty.
- **Plugin extractors**: Enabled plugins can add readers using PNG text chunk mappings, regex rules, JSON sidecar parsers, external programs or hidden plugin pages. Curio's optional plugins use this for AI generation parameters, EPUB/PDF/comic metadata (`pages`, `author`, `series`, `series_index`) and 3D model geometry stats. Plugin extractors also receive paths to any [grouped companion files](grouped-files.md) attached to the asset.
- **Precedence and text cap**: Built-in readers cap text values at `16 KB` to keep field indexes fast (plugins manage their own limits). When multiple readers fill the same key, the first reader in precedence order wins. Readers can also fill an asset's title, notes or URL when those fields are empty.

See [Register a metadata reader](../../plugins/contributions/extractors.md) to write a custom extractor.

## Indexing & rescan

You do not need to index every field across your whole library to inspect one file:

- **Inspect one asset on demand**: Selecting a single asset runs readers for switched-off fields on that file in memory and displays the results in the Inspector, marked **Not indexed**. Nothing is written to the database, and those values are not searchable until you turn the field on.
- **Enable indexing for searches**: Toggle a field on in **Settings › Fields** and choose **Scan now** to backfill existing files, or **Later** to index only new and changed files going forward.
- **Rescan selected assets**: Right-click selected assets and choose **More › Rescan metadata** to re-run active readers on those files. Manual overrides are preserved.
- **Rescan a group or field**: In **Settings › Fields**, click **Rescan** on any source card or expanded field row and choose **Missing only** or **Everything**. When a file's modification time changes on disk during a location scan, Curio re-queues metadata extraction automatically.
