# File actions

Curio separates **catalog edits** (always allowed in the library database) from **disk operations**. Disk operations depend on whether a file lives in managed `files/` storage or in an indexed location set to **Read-only**, **Edit files** or **Edit and organize**.

## Open & reveal

Right-click any asset (or selection) in the grid to access file actions:

- **Open** (++enter++ or double-click): Opens the asset in Curio's built-in viewer or a plugin viewer.
- **Open in a window** (or **Open these in a window** for multiple assets): Opens the selection in a standalone viewer window that can stay pinned **Always on top**.
- **Open with default app**: Launches the file in your operating system's default application.
- **Open with** (single asset): Lets you choose an external program to open the file. External programs can only be launched from Curio's window, never remotely through the API.
- **Show in file manager** (single asset): Opens your OS file manager with the file highlighted.
- **Copy › Files** / **Copy › Path**: Copies the files to the system clipboard (ready to paste into a file manager, chat or editor via [`clipboard.copyFiles`](../../api/operations/clipboard.md#clipboard-copyfiles)) or copies their full disk paths as text.

## Rename & replace

- **Rename** (++f2++ or edit **Name** in the Inspector):
    - **Managed (copied) assets**: Updates the display name and renames the file on disk in every folder hard link while keeping the file extension. If another file in a folder uses that name, Curio appends a number.
    - **Edit and organize locations**: Updates the display name and renames the file on disk.
    - **Edit files or Read-only locations**: Changes only the display name in Curio's database; the filename on disk stays untouched.
- **Duplicate** ([`item.duplicate`](../../api/operations/item-lifecycle.md#item-duplicate)): Creates an independent asset with its own copy of the file and copies all tags, folders, ratings, notes and custom fields.
- **More › Replace file** ([`item.replaceFile`](../../api/operations/item-files.md#item-replacefile)): Swaps an asset's file with another file of the same type while keeping its tags, folders, notes, ratings and custom fields. The previous file goes to the OS Recycle Bin or Trash. Unavailable in **Read-only** locations.

## Custom covers

Any asset (including videos, documents, archives, links and 3D models) can use a custom thumbnail cover:

- **More › Thumbnail from file** (++ctrl+shift+t++): Choose a PNG, JPEG or WebP image to replace the asset's thumbnail.
- **More › Thumbnail from clipboard**: Paste an image from the clipboard as the asset's thumbnail.
- Custom covers update the cached aspect ratio immediately so Masonry and Justified views reshape to fit. Choose **More › Refresh thumbnail** to revert to Curio's generated thumbnail.

## Refresh

Regenerate derived data for selected assets without re-importing them from the **More** submenu:

| Action | What it recalculates |
|---|---|
| **More › Refresh thumbnail** | Re-decodes the asset (or runs its plugin thumbnailer) to rebuild cached thumbnails and previews. |
| **More › Refresh colors** | Re-extracts the dominant color swatches from the asset's current thumbnail for **Color** filtering and **Spectrum** view. |
| **More › Rescan metadata** | Re-runs built-in and plugin file readers on the selected files while preserving manual overrides. |

## Trash & purge

- **Move to Trash** (++delete++): Moves selected assets into the library **Trash** view without erasing bytes on disk.
- **Restore**: Select assets in **Trash** and choose **Restore** to return them to their previous folders.
- **Empty trash / Delete permanently** ([`item.purge`](../../api/operations/item-lifecycle.md#item-purge)): Removes trashed assets permanently from the library database. For managed copies, Curio sends the files from `<library>.curio/files/.trash/` to the OS Recycle Bin or Trash instead of deleting them permanently.

## Folder trash

Pressing ++delete++ inside a folder affects only that folder when an asset belongs to more than one:

- **Asset in multiple folders**: Removes only that folder's entry (moving that folder's hard link to `files/.trash/`). The asset stays active in its other folders.
- **Deleting from All, a search or the asset's last folder**: Moves the entire asset to the library **Trash**.
- **Restore to a specific folder**: Dragging an asset out of **Trash** onto a sidebar folder restores it into that folder only instead of recreating its previous memberships.

## Disk effects

| Operation | Managed (copied) files in `files/` | Location (**Read-only**) | Location (**Edit files**) | Location (**Edit and organize**) |
|---|---|---|---|---|
| **Move to Trash** | Hard link moves to `files/.trash/` | Removed from active views (file untouched) | Removed from active views (file untouched) | Removed from active views (file untouched) |
| **Empty trash / Purge** | Managed file sent to OS Recycle Bin or Trash | Index entry deleted; original file **never** deleted | Index entry deleted; original file **never** deleted | Index entry deleted; original file **never** deleted |
| **Rotate file / Replace file** | Updates file in `files/` across all folder links | Refused | Modifies file in place | Modifies file in place |
| **Rename asset** | Renames file in `files/` | Index name only | Index name only | Renames file on disk |

!!! warning "Curio never deletes linked originals when emptying Trash"
    Emptying Curio's Trash only sends library-owned copies inside `<library>.curio/files/` to the OS Recycle Bin or Trash and removes folder hard links. Original files in indexed locations are **never** deleted from disk by Trash or Purge.
