# Formats

Curio indexes every file in a library, assigns a format family and selects a decoder or viewer for its extension. You can turn off individual built-in format families in **Settings › Plugins**. Optional plugins add dedicated viewers, thumbnails and metadata extractors for books, comics, PDFs, 3D models and AI generation metadata.

## By family

Each built-in format family appears as a toggle in **Settings › Plugins**. Turning a family off stops Curio from generating thumbnails, extracting metadata or opening those extensions in built-in viewers while keeping the files indexed as plain files.

| Family | Asset type | Extensions | Thumbnail source | Built-in viewer |
|---|---|---|---|---|
| **Images** (web-native) | Images | `jpg`, `jpeg`, `jfif`, `jpe`, `png`, `apng`, `gif`, `webp`, `avif`, `svg`, `bmp`, `ico` | Built-in decoder (FFmpeg fallback for `avif`) | Image viewer; animated GIF, WebP and APNG play directly |
| **Images** (other raster) | Images | `tif`, `tiff`, `tga`, `qoi`, `dds`, `hdr`, `pnm`, `ppm`, `pgm`, `icns`, `insp` | Built-in decoder (OS thumbnailer tried first for `dds`) | Decoded PNG preview at stage resolution |
| **Images** (modern & layered) | Images | `heic`, `heif`, `hif`, `jxl`, `psd`, `psdt`, `psb`, `exr`, `kra`, `xcf` | FFmpeg or OS thumbnailer | Decoded preview; sharpens when zooming |
| **Images** (camera RAW) | Images | `arw`, `cr2`, `cr3`, `crw`, `dng`, `nef`, `nrw`, `orf`, `raf`, `rw2`, `pef`, `srw`, `sr2`, `raw`, `3fr`, `erf`, `x3f`, `mrw` | Largest embedded camera preview, rotated from metadata | Embedded preview up to full embedded size |
| **Images** (vector / PostScript) | Images | `ai`, `ait`, `eps` | `pdftoppm` for PDF-compatible `ai`/`ait`, else OS thumbnailer or embedded EPS/AI preview | Decoded preview |
| **Video** (native playback) | Videos | `mp4`, `mov`, `m4v`, `webm` | FFmpeg frame at 1.5 s (or 25% into shorter clips), up to 512 px | Native WebView video player |
| **Video** (transcoded) | Videos | `mkv`, `avi`, `wmv`, `mpg`, `mpeg`, `mts`, `m2ts`, `ts`, `flv`, `f4v`, `3gp`, `ogv`, `ogm` | FFmpeg frame (OS thumbnailer fallback) | Converted once to a cached MP4 (remuxed if H.264, else H.264/AAC) |
| **Audio** | Audio | `mp3`, `wav`, `flac`, `ogg`, `oga`, `opus`, `aac`, `m4a`, `aiff`, `aif`, `wma` | Embedded cover art or sibling cover file (`cover`, `folder`, `front`, `albumart`); else icon | Audio player with cover art and cached waveform |
| **Documents** | Documents | `pdf`, `txt`, `md`, `markdown`, `csv`, `tsv`, `log`, `srt`, `vtt`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`, `potx`, `odt`, `ods`, `odp`, `rtf`, `key`, `pages`, `number`, `numbers` | `pdf`: Reader plugin, `pdftoppm` or OS thumbnailer; text: first 32 KB card; `key`: embedded preview; Office: OS thumbnailer | Text files open in the text viewer; `pdf` opens in Reader or the WebView PDF viewer; Office files open in the default app |
| **Code** | Code | `html`, `htm`, `mhtml`, `xhtml`, `json`, `jsonc`, `json5`, `xml`, `xsl`, `xslt`, `js`, `mjs`, `cjs`, `jsx`, `ts`, `mts`, `cts`, `tsx`, `css`, `scss`, `sass`, `less`, `yaml`, `yml`, `toml`, `ini`, `cfg`, `conf`, `env`, `sh`, `bash`, `zsh`, `fish`, `bat`, `cmd`, `ps1`, `psm1`, `py`, `pyw`, `rs`, `c`, `h`, `cpp`, `cc`, `cxx`, `hpp`, `hh`, `cs`, `java`, `kt`, `kts`, `go`, `rb`, `php`, `lua`, `sql`, `r`, `swift`, `dart`, `scala`, `ex`, `exs`, `erl`, `hs`, `ml`, `zig`, `nim`, `vue`, `svelte`, `astro`, `glsl`, `wgsl`, `hlsl`, `dockerfile`, `makefile`, `cmake`, `gradle`, `proto`, `graphql`, `gql` | First 32 KB of text drawn as a card | Common formats (`json`, `xml`, `yaml`, `yml`, `ini`, `toml`, `js`, `py`, `rs`, `css`, `scss`, `sh`, `bat`, `ps1`, `c`, `cpp`, `h`, `java`, `go`, `lua`, `sql`) open in the text viewer; others open in the default app |
| **Links** | Links | `url`, `webloc` | Web capture (when **Settings › Preview › Web links › Link previews** is on) or YouTube thumbnail | Live web page in an isolated WebView, or embedded YouTube player |
| **Design projects** | Documents | `xd`, `cdr`, `skp`, `dwg`, `clip`, `af`, `afpub`, `afdesign`, `afphoto`, `indd`, `indt`, `idml`, `prd`, `fig`, `sketch`, `pxd`, `skt`, `graffle`, `mindnode`, `xmind`, `eddx`, `emmx`, `ps` | Embedded preview (`sketch`, `xmind`, `idml`, `indd`, `indt`, `skp`, `dwg`) or OS thumbnailer | Default system app (thumbnail shown in viewer) |
| **Archives** | Archives | `zip`, `rar`, `7z`, `tar`, `gz`, `tgz`, `xz` | `zip` and `7z`: first image in reading order; others: icon | Default system app (or Reader plugin for comic archives) |
| **Fonts** | Fonts | `ttf`, `otf`, `ttc`, `otc`, `woff`, `woff2`, `dfont` | Three-line specimen drawn in the font's primary script | Built-in Font viewer (Sample text, Waterfall, Glyphs and Details) |
| **3D models** | 3D models | `glb`, `gltf`, `obj`, `stl`, `3mf`, `ply`, `fbx`, `3ds`, `dae`, `ifc` | **3D models** plugin (`curio.models`) for `stl`, `3mf`, `obj`, `glb`, `gltf` and `ply`; OS thumbnailer or icon otherwise | **3D models** plugin viewer for `stl`, `3mf`, `obj`, `glb`, `gltf` and `ply`; default app for others |
| **3D scenes** | 3D models | `blend`, `c4d` | OS thumbnailer, then embedded Blender preview (uncompressed and gzip `.blend`; zstd `.blend` has no decoded preview) | Default system app |

!!! note "Platform differences"
    - **Windows**: Curio bundles `ffmpeg.exe` and `ffprobe.exe` in `resources/ffmpeg/` and queries the Windows shell thumbnail cache for formats with registered desktop thumbnail providers.
    - **macOS**: Ad-hoc signed, non-notarized macOS builds (macOS 12 Monterey or later, Apple Silicon and Intel) bundle `ffmpeg` and `ffprobe`. macOS Quick Look supplies OS thumbnails for PDFs, Office, Pages, Keynote and installed Quick Look extensions.
    - **Linux**: Curio uses the system `ffmpeg` package and GStreamer codecs for playback. Linux has no OS thumbnailer fallback.

## Capabilities

Every indexed file is searchable by name and path, taggable and assignable to folders. Beyond indexing, each asset can use up to four pipelines:

1. **Thumbnails**: Background workers generate thumbnails, prioritizing assets visible on screen. Thumbnail resolution matches your cache setting (320 px, 480 px or 960 px).
2. **Viewer previews**: Opening a picture shows its cached thumbnail at fitted dimensions first, then swaps in a preview sized in 256 px steps (up to 8192 px). Zooming requests sharper pixels on demand; JPEGs use scaled DCT decoding for speed.
3. **Playback**: Video and audio play through the WebView's hardware-accelerated media engine. FFmpeg converts unsupported containers or codecs once into a cached playable MP4.
4. **Metadata extraction**: Built-in extractors read image EXIF and dimensions, video and audio encoding, audio tags (artist, album, title, track, disc, year, genre and composer) and font tables. Optional plugins supply additional fields.

## Images & RAW

Web-native images (`jpg`, `png`, `gif`, `webp`, `avif`, `bmp`, `ico`, `svg`) render directly in the viewer without transcoding. Curio detects animated `gif`, `webp` and `apng` files from their headers during import. Their thumbnails stay on the first frame while playback runs on hover or continuously according to **Settings › Controls › Animated images**.

SVG files render safely offline: inline `data:` images work, while external URLs and local file references are blocked. Untrusted SVGs cannot read local files or make network requests.

Camera RAW files (`cr2`, `cr3`, `nef`, `arw`, `dng`, `raf`, `orf`, `rw2` and related formats) extract the largest embedded preview stored by the camera and rotate it upright. Layered design files (`psd`, `psb`, `xcf`, `kra`, `afphoto`, `afdesign`, `afpub`, `clip`, `cdr`, `indd`, `sketch`, `fig`, `xd`) use FFmpeg, embedded previews or OS thumbnailers.

Curio automatically groups Live Photo `.mov` clips, `.aae` edit files and `.xmp` sidecars into their primary photo or video. Turn on **RAW + JPEG** grouping in **Settings › Grouped files** to pair RAW files with matching JPEG or HEIC files. See [Keep related files in one asset](../organize/grouped-files.md) and [Inspect pictures](images.md).

## Media & fonts

Video files in `mp4`, `mov`, `m4v` and `webm` play directly when the platform WebView supports their codecs. Other containers (`mkv`, `avi`, `wmv`, `flv`, `ts`, `mpg`, `3gp`, `ogv`) are remuxed or transcoded once by FFmpeg into a cached MP4. Audio files open in the built-in player with cover art, a cached waveform timeline and searchable audio tags. See [Play video and audio](media.md).

Font files (`ttf`, `otf`, `ttc`, `otc`, `woff`, `woff2`, `dfont`) read their OpenType tables directly without loading the full file into memory. Curio draws three-line script-aware thumbnails and opens fonts in the built-in Font viewer with sample text, waterfall, glyphs and variable axis sliders. Legacy Type 1 (`pfb`, `pfa`), bitmap (`fon`, `fnt`), `eot` and SVG fonts are not rendered because the WebView cannot load them. See [Preview fonts](fonts.md).

## Docs & archives

Without optional plugins, PDF files render their first-page thumbnail with `pdftoppm` (if installed) or the OS thumbnailer, and open in the WebView's built-in PDF viewer.

With the optional **Reader** plugin (`curio.reader`) enabled, Curio adds:

- **PDF** (`pdf`): Rendered cover thumbnails and an interactive reader with outline, search, bookmarks and reading progress.
- **Ebooks** (`epub`, `fb2`, `fbz`, `mobi`, `azw3`, `azw`, `prc`): Cover extraction, bibliographic metadata and paginated or scrolled reading.
- **Comics** (`cbz`, `cb7`): First-image covers in reading order, two-page spreads, continuous strip and page grids.

`zip`, `7z`, `cbz` and `cb7` archives use their first image in reading order as their thumbnail, with or without the Reader. Other archives (`rar`, `tar`, `gz`, `tgz`, `xz`) stay indexed and open in your system's default archive app. See [Read books, PDFs and comics](reader.md).

## Text, code & 3D

Curio separates prose documents (`.txt`, `.md`, `.markdown`, `.csv`, `.log`, `.srt`, `.vtt` under **Documents**) from source code and structured data (`.html`, `.json`, `.xml`, `.js`, `.ts`, `.css`, `.py`, `.rs` and similar formats under **Code**). Supported text files open in the built-in text viewer with line wrapping, line numbers, formatted JSON and Markdown modes. See [Read text, code and web links](text-links.md).

Saved web links (`.url` and `.webloc`) belong to the **Links** asset type. Link previews are off by default so scanning links does not contact external sites; turn on **Settings › Preview › Web links › Link previews** to capture page screenshots and YouTube thumbnails.

3D models (`glb`, `gltf`, `obj`, `stl`, `3mf`, `ply`, `fbx`, `3ds`, `dae`, `ifc`) and 3D scenes (`blend`, `c4d`) are separate format families so turning off 3D models keeps Blender and Cinema 4D thumbnails active. Files whose extension matches no built-in family or active plugin stay indexed under **Other** and open in their default app.

## Fallbacks

When generating thumbnails, Curio checks enabled plugin thumbnailers first, then embedded previews and OS thumbnails (Windows shell cache or macOS Quick Look). It then falls back to built-in decoders, FFmpeg, `pdftoppm` and finally a file-type icon.

Assets without a built-in or plugin viewer show their thumbnail or file-type icon in the viewer stage. Launch them in your system's default app with **Open with default app** (++shift+alt+o++) or by middle-clicking the viewer stage.

## Plugins

Optional plugins (installed via **Settings › Plugins**) add viewers, thumbnailers, custom fields and custom display types that appear in **Group by › Type**, the **Types** filter and the List **Type** column:

| Plugin | Adds support for | Custom type & features | Guide |
|---|---|---|---|
| **Reader** (`curio.reader`) | `pdf`, `epub`, `fb2`, `fbz`, `mobi`, `azw3`, `azw`, `prc`, `cbz`, `cb7` | Registers **Books** type, covers, bibliographic metadata, reading progress and interactive reader | [Read books, PDFs and comics](reader.md) |
| **3D models** (`curio.models`) | `stl`, `3mf`, `obj`, `glb`, `gltf`, `ply` | Interactive 3D orbit viewer, rendered thumbnails, geometry metadata and companion grouping | [Inspect 3D models](models.md) |
| **AI generation** (`curio.ai-generated`) | Images with generation metadata plus `.safetensors`, `.sft`, `.ckpt`, `.gguf` weights | Registers **Models** type, extracts prompts and paired LoRA strengths, groups previews/sidecars and opens matching images | [Inspect AI generation metadata](ai.md) |
