# Inspector

The Inspector shows details and editors for the current selection, or a summary of the current view when nothing is selected. Every section in the Inspector is an independent widget that you can collapse, reorder, duplicate or hide from **Edit layout** (see [Arrange the window](../customize/layout.md)). Press ++ctrl+alt+b++ to cycle the Inspector between full, compact and hidden, or press ++ctrl+shift+b++ to toggle both sidebars.

## Selection modes

The Inspector adapts automatically to how many assets are selected:

| Selection state | What the Inspector shows |
|---|---|
| **Nothing selected** | **Summary**: current view title (marked `· filtered` when a search or filter is active), asset count, total file size, breakdown by asset type and unavailable count when viewing **All**. In **Spectrum** view, the **Spectrum map** minimap also appears. |
| **One asset selected** | Single-asset widgets: **Preview**, **Colors** palette, **Name**, **Notes**, **URL**, **Rating** (stars, Favorite, Explicit), **Tags**, **Folders**, **Files** (when grouped), **Properties** (plus visible custom fields) and applicable format or plugin widgets (Camera, Audio, Video, Font, AI generation, Book, 3D model and custom single-field widgets). |
| **Several assets selected** | **Preview** shows a fanned stack of the last three selected assets with the selection count. **Notes**, **URL**, **Rating**, **Tags**, **Folders** and **Properties** switch to multi-selection mode, and **Batch edit** appears at the bottom for setting a field or trashing and restoring the selection. |

Widgets that do not apply to the current selection hide automatically instead of showing empty rows. Examples include **Colors** on an asset with no palette, **Duration** on a still image, **Files** on an ungrouped asset and format widgets on other file types. Widgets with headings (**Tags**, **Folders**, **Properties**, **File & location**, **Files** and headed plugin widgets) remember their collapsed state per layout. **Camera** and **File & location** start collapsed by default.

## Core fields

In the default Inspector layout, the primary editable fields sit directly under the preview and color palette:

- **Name** (single selection): Edits the asset's display name while keeping its file extension attached. Press ++f2++ with one asset selected to focus this box, then click away or press ++enter++ to save. Clearing the box restores the previous name.
- **Notes** (single or multiple): Multi-line text box saved when you click away. When selected assets have different notes, the placeholder reads *"Different notes; typing sets them for all"* and leaves existing notes untouched unless you type a new value.
- **URL** (single or multiple): Source web address, with an **Open in browser** button whenever a URL is set. Click away or press ++enter++ to save. When selected assets have different URLs, the placeholder reads *"multiple URLs"* and leaves existing values untouched until edited.
- **Rating** (single or multiple): Sits below **URL** and above **Tags** in a single row with no heading and three controls:
    - **Stars (1 to 5)**: Click a star to set the rating, or click the active star on a single asset to clear it to unrated (`0`). On a multi-selection, stars display only when every selected asset shares the same rating. Press ++0++ through ++5++ to rate the selection from the grid.
    - **Favorite** (heart button): Marks or unmarks the selected assets as favorites. Lights up on a multi-selection when all selected assets are favorites. Bind **Toggle favorite** in **Settings › Shortcuts** (unbound by default).
    - **Explicit** (shield button): Marks or unmarks the selected assets as explicit. Explicit assets blur their thumbnails in Grid, Masonry, Justified, List and Spectrum views while staying unblurred in the Inspector preview and viewer. Bind **Toggle explicit** in **Settings › Shortcuts** (unbound by default).

## Tags & fields

- **Tags**: Shows all tags on one asset, or tags shared by **all** selected assets. Click **+** (or press ++t++ / ++ctrl+k++) to open the tag picker. Click a chip to filter by that tag, or middle-click or click its remove button to remove it. Right-click a chip to **Rename tag** (globally), **Copy tag**, **Paste tags** or **Remove**. Right-click the **Tags** heading to **Copy tags**, **Paste tags** or **Remove all tags**.
- **Folders**: Shows every folder containing the asset (or containing **all** selected assets). Click **+** (or press ++f++ / ++ctrl+shift+d++) to open the folder picker. Click a folder chip to open that folder. Middle-click a chip or click its remove button to remove the selection from that folder without deleting the asset.
- **Custom and plugin fields**: Visible custom fields appear at the bottom of **Properties**, and any field can be placed as its own widget in **Edit layout**. Plugin and built-in reader widgets render populated fields as type-aware editors or read-only rows:
    - Click a read-only value to filter the library by that exact value, or right-click it to copy the value.
    - Multi-value read-only fields and paired fields (such as LoRAs with strengths) stay as individual clickable rows with their paired value in parentheses.
    - Long text, code and JSON fields open in a vertically resizable box with **Find items** (searches selected text, or the first 60 characters if none is selected) and **Copy** buttons.
    - When you edit a file-backed field by hand, a pencil-check button appears beside its label. Click it to **Restore file value** so metadata readers fill it again.
    - Fields switched off in **Settings › Fields** still show values read on demand from the file in read-only mode, marked **Not indexed** (not clickable for filtering).

## Properties

The **Properties** widget lists core file metadata for one asset:

- **Dimensions**: Pixel width × height, shown only for image and video files (not thumbnails of books, comics, PDFs or web links).
- **Duration**: Formatted as `MM:SS` or `HH:MM:SS` for audio and video files.
- **Size**: Formatted file size.
- **Type**: Uppercase file extension.
- **Date added**, **Date created** and **Date modified**: Local timestamps (`YYYY/MM/DD HH:MM`).

For multiple selected assets, **Properties** condenses to total **Size**, combined **Duration** (when any selected asset has duration) and a **Types** breakdown (for example `4 images, 2 videos`).

Each property is also available as a standalone single-row widget in **Edit layout** under **Single properties**: **Dimensions**, **Duration**, **Pages** (from PDF, book or archive plugins), **File size**, **File type**, **Added**, **Date created** and **Date modified**.

## Preview & palette

- **Preview**: Displays the selected asset with its file extension badge in the corner, or a fanned three-card stack for multiple selected assets. Drag the thin handle beneath the preview to resize its height between `80 px` and `800 px` (saved per layout).
    - Animated GIF, WebP and APNG files show their static first-frame thumbnail with no hover playback, so hovering the Inspector does not disrupt playback in the viewer.
    - Video thumbnails display without a centered play-icon overlay.
    - Assets marked **Explicit** stay unblurred in the Inspector preview.
    - Offline or missing files show their cached thumbnail (if available) with a **Storage offline** or **File missing** badge.
- **Colors**: Displays the asset's dominant color bands proportional to each color's area (omitting slivers under 3% except the primary color). Hover a band to see its hex code and percentage, left-click to filter by that color or right-click to copy its hex value.
- **Custom cover**: To replace an asset's thumbnail from a file or the clipboard, use **More › Thumbnail from file**, **More › Thumbnail from clipboard** or press ++ctrl+shift+t++. See [File operations and trash](file-actions.md).

## File & parts

- **File & location** (single selection, collapsed by default; add it from **Edit layout** if hidden): Shows **Status** (**Available**, **Storage offline** or **Missing**) and **Storage** (**Copied into the library** or **Linked in &lt;location&gt; · &lt;access&gt;**). Also shows the full path on disk and the first 16 characters of its `BLAKE3` content hash when hashed. Action buttons let you **Show in file manager**, **Copy path** or **Relink** (when offline or missing).
- **Files** (shown for grouped assets): Lists the primary file first, then each member with its role (`metadata`, `preview`, `buffers`, `materials` or a plugin-defined role) and size, plus the combined **Total** size. For each member you can **Open**, **Show in file manager**, **Detach** (turns it into a standalone asset) or **Forget** (for missing or offline members). You can also drag a member row out of Curio to another app, or click **Split** to separate all members into individual assets. See [Keep related files in one asset](grouped-files.md).

## Bulk & export

When two or more assets are selected:

- **Batch edit**: Provides **Set field** (a searchable dropdown of indexed fields that apply to every selected asset's file type, with **Clear for all selected**) and a lifecycle button: **Move all to trash** in normal views, or **Restore all** and **Delete all permanently** in **Trash**.
- **Export button** (available from **Edit layout**): Works on one or multiple selected assets with a dropdown for **Export files**, **Metadata as JSON** and **Metadata as CSV**. See [Export files and metadata](../import/export.md).
