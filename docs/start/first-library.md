# First library

This walkthrough covers creating a library, adding files, tagging assets and running your first search.

## Library location

Every Curio library is a self-contained folder ending in `.curio` (for example `Design.curio` or `Photos.curio`). Inside it are the SQLite database (`library.db`), cached thumbnails, automatic backups and a `files/` folder tree.

Create your first library on a **local disk**, such as your internal drive.

!!! warning "Keep the `.curio` library folder on a local disk"
    Curio's SQLite database requires reliable local file locking. Do not place a `.curio` library folder inside a live cloud-sync folder (OneDrive, Dropbox, Google Drive) or open the same `.curio` folder from two computers over a network share at the same time. If your media lives on a NAS or external drive, keep the `.curio` folder on your local disk and add the NAS folder as an **indexed location** instead. See [Libraries in multiple windows](../features/libraries/windows.md).

You can change the default parent folder for new libraries in **Settings › General**.

## Add files

On first launch, the **Welcome** screen offers two starting actions:

- **Create new library**: Choose a name and parent folder to create a `<Name>.curio` library.
- **Open an existing library**: Open an existing `.curio` folder on disk.

Once your library opens, decide how to bring files in. You can mix both storage modes in the same library:

=== "Copy files into the library (managed)"

    Best when you want a self-contained archive of references, screenshots, downloads or inspiration.

    1. Drag files or folders from your file manager into the main grid, or open the **+** menu and choose **Import files** or **Import folder**.
    2. Curio copies the files into `<Library>.curio/files/` and mirrors your dropped folders as library folders.
    3. Your original files outside the library remain untouched.

=== "Index an existing folder in place (location)"

    Best for photo archives, game asset directories, font collections or NAS shares whose folder layout you want to keep.

    1. Click **+** next to **Locations** in the left sidebar (or open **Settings › Storage** and click **Add location**).
    2. Select a folder on disk. Curio indexes it in place without copying your originals.
    3. By default, Curio watches the folder for changes. Set its access level to **Read-only**, **Edit files** or **Edit and organize** to control what Curio may modify on disk.

For how both modes work on disk, see [Copied files and indexed locations](../features/libraries/storage.md) and [Add and manage locations](../features/libraries/locations.md).

## Indexing

When you add files or a location, Curio indexes them in the background while keeping the window responsive:

1. **Discovery and hashing**: Curio records each file's size, timestamps, type and dimensions, then adds assets to the grid right away.
2. **Thumbnails and color palettes**: Background workers build cached thumbnails and extract dominant color palettes. Active work shows as a persistent status notice with a cancel button.
3. **Metadata readers**: Built-in and enabled plugin readers extract EXIF camera info, audio/video streams, font metrics and any custom fields you enabled.

Click **All** in the left sidebar to see every asset in the library, or expand **Locations** or **Folders** to browse the structure you imported.

## Tags & folders

An asset can belong to multiple folders and carry any number of tags at the same time.

1. **Create a folder**: Click **+** next to **Folders** in the left sidebar, type a name such as `References` and press ++enter++.
2. **File an asset**: Drag assets from the grid onto `References` in the sidebar, or select them and press ++f++ to open the folder picker. Curio mirrors the folder inside `<Library>.curio/files/References/` using hard links (or copies when linking across drives).
3. **Add a tag**: Select an asset, press ++t++ (or click inside **Tags** in the **Inspector**), type a tag such as `landscape` and press ++enter++. Typing a new name creates the tag immediately.
4. **Rate or favorite**: In the Inspector's **Rating** row, click a star (1–5), click the heart for **Favorite** or toggle **Explicit** to blur its thumbnail in the grid. You can also press ++1++ through ++5++ (or ++0++ to clear) while assets are selected.

## Search & view

1. Press ++ctrl+f++ (or click the search box in the toolbar) and type part of a filename, note or tag. Curio matches prefixes as you type.
2. Combine text search with chips in the **Filter bar** (toggle with ++ctrl+shift+l++ or the funnel icon), such as **Color**, **Types**, **Rating** or **Tags**.
3. Type structured queries directly into the search box, such as `tag = landscape AND rating >= 4` or `ext = png AND width >= 1920`. See [Search the library](../features/search/text.md) and [Query language](../api/query/syntax.md).
4. **Open an asset**: Double-click an asset (or select it and press ++enter++ or ++space++) to open Curio's viewer. Press ++left++ or ++right++ to step through results, scroll to zoom and press ++escape++ or **Back** to return to the grid.

## Reopen

- **Switching libraries**: Click the library name in the top-left switcher to switch libraries, open a second library in its own window or create a new library.
- **Reopening on launch**: By default, Curio reopens your last library at startup. Configure startup behavior and tray or menu bar residency in **Settings › General**.
- **Closing**: Close the window or quit from the tray or menu bar. Background workers finish their current write transaction before exit so the SQLite index stays intact.

Next, explore the interface in [Find your way around](window.md).
