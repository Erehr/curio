# Locations

A **location** is an existing folder on your computer, external drive or mounted network share that Curio indexes in place. Your files stay where they are on disk while gaining Curio's search, tags, ratings, custom fields, library folders and smart folders.

## Add location

Add a location in any of these ways:

- Click the **+** (**Add**) button in the sidebar header and choose **Add location**.
- Click the **+** button next to the **Locations** heading in the sidebar.
- Open **Settings › Storage › Locations** and click **Add location**.
- Choose **Add Eagle library as location** from the **Add** menu or **Settings › Storage › Import** to index an Eagle `.library` in place (see [Bring an Eagle library into Curio](../import/eagle.md)).

The location appears under **Locations** in the sidebar with a spinning icon during its first scan. Assets stream into the library in batches as the directory tree is walked so you can browse right away. Thumbnails and metadata extraction follow once the scan completes so directory traversal stays fast.

!!! note "Locations vs. Uncategorized"
    Assets indexed inside a full location already live in their own disk directory tree, so they are excluded from **Uncategorized**. The **Uncategorized** view lists only managed assets and individually linked files that belong to no Curio folder.

Reorder locations by dragging them in the sidebar, pin a location to **Pinned** with **Pin** in its context menu or customize its icon and color dot.

## Directory tree

Every location with subdirectories shows an expand arrow in the **Locations** sidebar section:

- Click the location row to view all indexed assets in that location.
- Expand the location to browse its nested disk directories with asset counts.
- Click any subdirectory to open that directory scope with clickable toolbar breadcrumbs and subfolder cards above the grid.
- Use **Show subfolders** and **Show subfolder contents** in the Layout popover (or grid context menu) to switch between direct files and recursive contents.
- Right-click a location or any subdirectory inside it and choose **Copy into the library** to bring all assets under that path into managed storage.

## Access levels

Each location has an access level that controls what Curio may write on disk inside that folder. Change it from the location's context menu (**Files here**) or under **Settings › Storage › Locations**:

| Access setting | API value | What Curio may do in this location |
|---|---|---|
| **Read-only** | `read_only` | Read and index files only. Curio never modifies, rotates, replaces, renames, moves or writes sidecars in this location. |
| **Edit files** *(default)* | `full` | Everything in Read-only, plus editing file contents in place when asked (rotating pictures, replacing a file, saving video frames, writing `.curio.json` sidecars). Curio never renames or moves files. |
| **Edit and organize** | `organize` | Everything in Edit files, plus renaming the file on disk (and its grouped companions) when you rename the asset in Curio, and moving files via **Move into the library** or **Move to folder on disk**. |

The library's own managed store (`files/`) is always writable and cannot be restricted to **Read-only**.

!!! tip "Read-only media hint"
    If Curio detects that a location sits on read-only media, **Settings › Storage › Locations** shows a `Read-only media` hint recommending **Read-only** access. The choice stays yours.

## Watching & rescans

Curio keeps locations in sync with changes made outside the app in two ways:

1. **Real-time file watching**: On by default under **Settings › General › Indexing & hashing › Watch folders**. Toggle per location from its context menu (**Watch for changes** / **Stop watching for changes**). Curio uses native OS file-change notifications and falls back to a full rescan if notifications overflow or fail.
2. **Startup and manual rescans**: Curio reconciles all enabled locations whenever a library opens. Trigger a rescan anytime from a location's context menu (**Rescan**), or rescan one or all locations (**Rescan all**) under **Settings › Storage › Locations**.

Unchanged files with matching size and modified time are skipped without hashing or database writes. Modified files keep their asset identity and tags while refreshing their cached thumbnails and hashes.

### Indexed types

By default, a location indexes extensions from Curio's built-in format families (images, video, audio, documents, code, links, archives, fonts and 3D models). It also indexes extensions registered by enabled plugins and companion files matching active grouping rules.

To index every file in your locations regardless of extension, enable **Settings › General › Indexing & hashing › Index all file types in locations** and rescan. Files without a preview handler still appear in the library and can be tagged, rated, organized and opened with their default system app.

## External drives

Curio checks each location's storage volume and labels special drive types under **Settings › Storage › Locations**:

- **Removable**: USB drives and SD cards that may be unplugged between sessions.
- **Network**: Mounted SMB/UNC shares and network drives that may disconnect or be modified by other computers.

Every `30` seconds, Curio checks whether enabled locations went offline or came back online:

- **Disconnected drive or share**: Curio marks the location **Offline**, pauses its watcher and sets its assets to **Offline** instead of **Missing**. Tags, ratings, thumbnails and metadata stay visible.
- **Empty mount point**: If a mount point exists on disk as an empty directory where files were previously indexed, Curio treats it as **Offline** instead of marking its files deleted.
- **Reconnecting**: When the drive or share becomes readable again, Curio reconciles the location, restores its assets to normal status and resumes watching.

!!! tip "Network folders work best from a local library"
    Keep your `.curio` library folder on a local SSD and add your NAS or file share as a **location**. Never store the `.curio` library database on a network share for multi-computer use; see [Libraries in multiple windows](windows.md).

## Offline & relocate

Curio separates an unavailable drive from individual vanished files:

- **Offline**: The location root is unreachable (unplugged drive, disconnected share or empty mount point). Its assets show as offline in the **Missing & offline** sidebar view and return automatically when the drive reconnects.
- **Missing**: The location is online, but a specific file disappeared from its recorded path. Curio keeps the asset record along with its tags, notes, folders and thumbnails.
- **Automatic move and rename tracking**: When a file moves or is renamed inside a location, Curio matches it first by filesystem file identity. If that fails, Curio matches by partial hash (file size plus the first and last `64 KiB`). Matched files keep their full identity, tags, folders and history.
- **Relocate a moved location**: If an entire location moved to a new path or changed drive letter, right-click it in the sidebar (or open **Settings › Storage › Locations**) and choose **Relocate**. Pick the new path; Curio updates the location root and rescans without losing asset IDs, tags or metadata.
- **Locate a single missing file**: Right-click a missing asset and choose **More › Locate missing file** to point it to its new file on disk.

## Remove

Right-click a location in the sidebar and choose **Remove from library** (or select multiple locations and press ++delete++), then confirm.

!!! warning "Removing a location deletes its library metadata"
    Every asset belonging to that location leaves the library along with its tags, ratings, notes and custom field values. Any hard links or copies placed in `<library>.curio/files/` for library folders are queued for cleanup. Original files on disk in the location folder are never deleted or modified.

To keep the assets and their Curio metadata before disconnecting a folder, run **Copy into the library** on the location first. See [Move files and transfer assets](transfers.md).

## Related

- [Copied files and indexed locations](storage.md): Comparing locations with managed storage.
- [Move files and transfer assets](transfers.md): Copying or moving location assets into the library.
- [Locations API](../../api/operations/location.md): `location.add`, `location.setAccess`, `location.rescan`, `location.relocate` and `location.remove`.
