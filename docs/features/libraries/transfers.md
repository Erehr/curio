# Transfers

Move files between managed storage and indexed locations, or transfer assets between separate libraries, while keeping your metadata intact.

## Import to managed

Select one or more linked-in-place assets, right-click and open the **Storage** submenu:

- **Copy into the library**: Copies the file into `<library>.curio/files/` (in the asset's first folder, or at the top level if unfiled) and switches the asset to managed storage. The original stays untouched in its location, and Curio remembers it so future scans do not re-index it as a duplicate.
- **Move into the library**: Moves the original file out of its location into `<library>.curio/files/` and switches the asset to managed storage. This option appears only when at least one selected linked asset belongs to a location set to **Edit and organize**. Curio asks for confirmation before moving.

Bring in an entire location or any subdirectory inside it at once by right-clicking it in the **Locations** sidebar section and choosing **Copy into the library**.

!!! note "Eagle locations use hard links on the same drive"
    When you run **Copy into the library** on assets from an Eagle `.library` added as a location, Curio uses hard links whenever the Eagle library and your `.curio` library share a volume, taking no extra disk space.

## Move to disk

Right-click selected assets and choose **Storage › Move to folder on disk**. Use this to move managed files into an indexed location or move linked files between disk directories.

Pick a destination folder inside one of your library's existing locations. The destination cannot sit outside your locations or inside a **Read-only** location:

- **Managed assets**: The file leaves `<library>.curio/files/` and moves to the chosen disk folder, and the asset becomes linked in place. It keeps all of its library folders, and its entries in `files/` become links to the moved file.
- **Linked assets**: Moving between disk directories requires the source location to be set to **Edit and organize**.

### Safe moves

Curio journals every storage move in the database before touching files on disk:

- **Same drive**: Curio performs an atomic rename and never overwrites an existing file.
- **Across drives**: Curio copies the file to the destination, verifies the copy, updates the database record and then sends the original to the OS Recycle Bin or Trash.
- **Crash recovery**: If Curio crashes or power fails mid-move, the next open inspects the journal and completes or rolls back the operation.

## Between libraries

Transfer assets between two Curio libraries in three ways:

1. **Context menu (no second window needed)**: Select assets (or right-click a folder in the sidebar) and choose **Copy to another library › `<Library>`** or **Move to another library › `<Library>`**.
2. **Drag and drop between windows**: Open both libraries side by side and drag assets from one window onto the other window or onto a sidebar folder.
3. **Copy and paste across windows**: Press ++ctrl+c++ on assets in one library window, switch to the second window and press ++ctrl+v++.

When you choose **Move to another library**, Curio adds the assets to the destination library first, then moves the originals in the source library to its **Trash**. Originals are not removed from disk until you empty the source library's Trash.

### Destination storage

| Where the transferred file lives | What the receiving library does |
|---|---|
| Inside the source library's managed store (`.curio` or `.library`) | Copies the file into the receiving library's `files/` folder because that file belongs to the source library |
| Inside an external disk folder (indexed location) | Links the file in place in the receiving library unless you copied it explicitly |

If the destination library is open in another window, Curio sends the transfer directly to that window. If the destination library is closed, Curio acquires its lock temporarily to write the incoming records and then releases it. Thumbnails for the newly arrived assets are generated the next time that library opens.

## Preserved metadata

What travels with an asset depends on whether the transfer stays inside one library or crosses into another library:

| Metadata / structure | Within one library (storage transfer) | Between two libraries |
|---|---|---|
| Tags, rating, favorite, explicit, notes, source URL, display name | Preserved (same asset record) | Copied automatically (cached for `1` hour during drag-and-drop or copy-paste) |
| Custom fields | Preserved | Copied for every field whose `key` exists in the receiving library |
| Library folder membership | Preserved; **Copy into the library** or **Move into the library** into a specific folder also adds the asset to that folder | Not copied because folder IDs belong to one library; dropping onto a folder in the second window files incoming assets there |
| Grouped companion files | Moved or copied alongside their primary asset, keeping their stem-aligned names | Sent with the primary asset and automatically re-grouped on arrival |

!!! note "Sidecar metadata fallback after an hour"
    OS drag-and-drop and clipboards carry only file paths, so Curio caches outgoing asset metadata for `1` hour for the receiving window to apply. If an Eagle `.library` file arrives after an hour, or was dragged directly from a file manager, Curio falls back to the metadata stored beside it.

## Conflicts & limits

Transfers run safety checks before modifying any files:

- **Source access restrictions**: Moving a linked file out of its location (**Move into the library** or **Move to folder on disk**) is refused if the source location is **Read-only** or **Edit files**. Switch the location to **Edit and organize** first, or use **Copy into the library**. Moving files into a **Read-only** location is also refused.
- **Trashed assets**: Storage transfers skip assets in the Trash and report them in the outcome notice. Restore them first before changing their storage mode.
- **Name conflicts**: If the destination directory already has a file with the same name, or a name needed by a grouped companion file, Curio appends a number using your **Duplicate names** style (`(2)`, `#2` or `- 2`) instead of overwriting anything.
- **Missing or offline files**: Missing or offline source files cannot be transferred between libraries and are reported as errors. If a destination library folder no longer exists on disk, a cross-library transfer stops before changing the source library.
- **Partial group failures**: If a primary asset moves while a companion is locked or missing, the companion stays in place and remains grouped with the asset. Curio reports the failure in an error toast.

## Related

- [Copied files and indexed locations](storage.md): How managed storage and hard links work.
- [Add and manage locations](locations.md): Configuring **Read-only**, **Edit files** and **Edit and organize** access.
- [Libraries in multiple windows](windows.md): Working with two library windows side by side.
- [Transfer, export and remove API](../../api/operations/item-lifecycle.md): `item.copyToLibrary` and `item.moveToLibrary`.
