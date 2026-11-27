# Fonts

Curio includes built-in font inspection, script-aware thumbnails, a live specimen viewer with variable-axis controls, a List **Font sample** column and searchable typographic fields. **Fonts** is enabled by default in **Settings › Plugins**. Turning it off keeps font files in your library as plain files without font thumbnails, fields or the Font viewer.

## Font files

Curio parses only the required OpenType and TrueType tables from disk without loading full font files into memory.

| Format | Extensions | How Curio reads it |
|---|---|---|
| **TrueType and OpenType** | `.ttf`, `.otf` | Read directly |
| **Font collections** | `.ttc`, `.otc` | First face used for thumbnail and library fields; every face selectable in the Font viewer |
| **Web fonts** | `.woff`, `.woff2` | Decompressed and read directly |
| **macOS font suitcases** | `.dfont` | First `sfnt` resource read for thumbnail, fields and viewer (not drawn in the List **Font sample** column) |

Legacy PostScript Type 1 (`.pfb`, `.pfa`), Windows bitmap fonts (`.fon`, `.fnt`), Embedded OpenType (`.eot`) and SVG fonts are not rendered because the WebView cannot load them.

!!! tip "Browse your installed system fonts without copying them"
    Add your operating system's font folder (`C:\Windows\Fonts` on Windows, `~/Library/Fonts` on macOS or `/usr/share/fonts` on Linux) with **Add location**, then set that location to **Read-only** in **Settings › Storage**. Curio indexes installed fonts in place without modifying them and updates automatically when fonts are installed or removed.

## Specimen

Opening a font switches to the **Fonts** viewer screen, which has its own layout and the **Font** Inspector widget. Switch between four views from the toolbar or keyboard shortcuts:

- **Sample text** (++s++): Shows the family and style header, an editable title (1.8x size), a synced sample line and an editable body paragraph. Right-to-left layout applies automatically for RTL scripts.
- **Waterfall** (++w++): Renders the active sample line across 16 fixed sizes from 120 px down to 10 px (`120`, `96`, `72`, `60`, `48`, `42`, `36`, `32`, `28`, `24`, `21`, `18`, `16`, `14`, `12`, `10`).
- **Glyphs** (++y++): Shows every mapped character in a virtualized grid so large CJK fonts stay responsive. Click any cell to inspect a 120 px preview, Unicode code point, PostScript glyph name and a **Copy** button.
- **Details** (++i++): Lists full font metadata, including family, style, PostScript name, version, clickable designer and license URLs, format and glyph counts. It also shows scripts, category, weight, width, variable axes, named styles, OpenType features, color glyph tables, embedding rights and license text.

Additional Font viewer toolbar controls:

- **Text size** (`12–160 px`, shown in **Sample text** and **Glyphs**): Adjusts specimen size and glyph cell dimensions.
- **Appearance**: Chooses the reading backdrop (**Theme**, **Paper**, **Ink**, **Blue** or **Yellow**). For variable fonts, it also shows a **Named style** dropdown and live sliders for every non-hidden variable axis (`wght`, `wdth`, `slnt`, `ital`, `opsz` or custom axes).
- **Face** (shown for `.ttc` and `.otc` collections with multiple faces): Switches which face is loaded and inspected.

Curio remembers your chosen view, text size and backdrop across fonts.

## Sample text

Font thumbnails draw three lines dark-on-paper across both light and dark themes, showing capitals, lowercase letters and digits in the font's primary script:

- Fonts with thousands of CJK ideographs render Chinese, Japanese or Korean specimens.
- Fonts whose family name identifies a script (such as *Noto Naskh Arabic*) render that script.
- General-purpose fonts render Latin when supported, or whichever script they cover best.
- Symbol and icon fonts show a grid of their first mapped characters. Missing characters are omitted instead of showing fallback boxes.

!!! note "Headless imports and font thumbnails"
    Because font thumbnails render inside Curio's window engine, fonts added while no Curio window is open wait until a window opens before their thumbnails are generated.

Curio also keeps a shared sample text string across the window:

- Place the **Font sample text** widget in any toolbar or sidebar from the layout editor while **Fonts** is enabled.
- Typing in that widget, or editing the sample line inside **Sample text**, updates **Sample text**, **Waterfall** and every row of the List **Font sample** column together.
- When left empty, the List column displays *"The quick brown fox jumps over the lazy dog"* while the Font viewer uses each font's script-appropriate pangram.

## Columns & fields

In **List** view, click the `+` button on the column header to enable the **Font sample** column. It loads each visible row's font file on demand (caching the last 200 loaded faces) and scales with the List row size.

Scanning fonts populates typographic fields in the library index. Fields marked *Offered (off)* start disabled and can be turned on in **Settings › Fields** followed by a rescan:

| Field | Query key | Enabled by default | Source |
|---|---|---|---|
| **Font family** | `font_family` | Offered (off) | Typographic family, falling back to family name |
| **Font style** | `font_style` | On | Typographic style, falling back to style name (`Regular`, `Bold Italic`) |
| **Font weight** | `font_weight` | On | CSS weight `100–900` (corrects fonts reporting `250` for Thin or ExtraLight) |
| **Font width** | `font_width` | On | `Ultra condensed` through `Ultra expanded` |
| **Italic** | `font_italic` | On | Italic or oblique flags or style name |
| **Font category** | `font_class` | On | Detected category (Serif, Sans serif, Slab serif, Monospace, Display, Handwriting, Symbol) |
| **Scripts** | `font_scripts` | On | Detected writing systems |
| **Variable axes** | `font_axes` | On | Axis names (`Weight`, `Width`, `Slant`, `Italic`, `Optical size` or custom) |
| **Glyphs** | `glyphs` | Offered (off) | Total glyph count |
| **Faces** | `font_faces` | Offered (off) | Face count in a collection (when greater than 1) |
| **Designer** | `designer` | On | Font designer name |
| **Foundry** | `foundry` | Offered (off) | Font foundry or vendor name |
| **Font version** | `font_version` | Offered (off) | Cleaned version string |
| **Font license** | `font_license` | On | Short SPDX-style name (`OFL-1.1`, `Apache-2.0`, `MIT`, `GPL`) or first license sentence |

Enabled font fields appear in the Inspector's **Font** widget, the **Filter bar** (**Font category**, **Scripts** and **Font weight**), sort and group menus, search queries and smart folders:

```text
font_class = Serif AND font_scripts = Cyrillic AND font_weight >= 600
font_axes IS NOT EMPTY
font_weight = 700 AND font_italic = true
```

## Glyphs & scripts

Curio determines typographic categories and script coverage from the font's glyphs and OpenType tables rather than trusting header flags alone.

### Category detection

Curio evaluates font categories in priority order and leaves the category blank instead of guessing when none match:

1. **Symbol**: Symbol character map only, PANOSE *Latin Pictorial* or OS/2 *Symbolic* class.
2. **Monospace**: Fixed-pitch flag, PANOSE *Monospaced* or identical advance widths for `i`, `m`, `W` and `0` (exempted for CJK fonts whose Latin subset is fixed-width).
3. **Handwriting**: PANOSE *Latin Hand Written* or OS/2 *Scripts* class.
4. **Display**: PANOSE *Latin Decorative* or OS/2 *Ornamentals* class.
5. **Family name keywords**: Words like `symbol`, `icons`, `emoji`, `dings` (Symbol); `mono`, `code`, `courier`, `console` (Monospace); `script`, `hand`, `brush`, `calligraphy` (Handwriting); `stencil`, `ornament` (Display).
6. **PANOSE serif style**: Square serifs map to **Slab serif**; cove, thin, oval, exaggerated and triangle serifs map to **Serif**; sans styles map to **Sans serif**.
7. **OS/2 family class**: Clarendon and slab serifs, other serifs or sans serif.
8. **Remaining family name keywords**: `slab`; `sans`, `grotesk`, `gothic`, `hei`; `serif`, `roman`, `mincho`, `song`.
9. **Outline geometry**: For Latin fonts, compares the outline width of `I` and `l` at the foot versus mid-height (at least 1.5x wider at the foot classifies as **Serif**).

### Script detection

A script is recorded only when a font maps at least 90% of that script's basic letters (50% for Indic scripts with sparse ranges): Latin, Greek, Cyrillic, Armenian, Georgian, Hebrew, Arabic, Thai, Lao, Khmer, Myanmar, Tibetan, Ethiopic, Devanagari, Bengali, Gurmukhi, Gujarati, Tamil, Telugu, Kannada, Malayalam and Sinhala. Korean requires 2,000 Hangul syllables, Japanese requires kana and Chinese requires 2,000 ideographs, using OS/2 code-page bits to disambiguate CJK when present.
