# Drag & drop

Curio shows a live drop preview while you drag so you know whether a drop will **link**, **move**, **copy**, **tag**, **restore** or **open** files. Dragging an indexed asset onto a folder never moves or renames its file on disk unless the location grants **Edit and organize** access and you confirm.

## From desktop

| Drag source | Drop target | Disk effect | Library effect | Drop preview |
|---|---|---|---|---|
| Files or folders from OS file manager | Main grid (outside a folder) | New files copy into `<library>.curio/files/`; files already in this library are not duplicated | New assets added; dropped directories become nested Curio folders | *Copy into the library* |
| Files or folders from OS file manager | Sidebar folder, Pinned folder, subfolder card or grid inside a folder | Same as above | New assets added and filed into that folder (receiving its auto-tags); already-known assets join that folder | *Copy into \<folder\>* |
| Files from another `.curio` library's `files/` folder | Curio window | Always copied (libraries never share hard links) | New assets in this library | *Copy into the library* |
| One `.curio` library folder | Curio window | Untouched | Opens that library | *Open this library* |
| One Eagle `.library` folder | Curio window or folder | Hard-linked on the same drive, or copied across drives | Imports its assets, folders, tags and smart folders | *Copy into the library* |

Indexed locations in the sidebar are not drop targets for desktop files. To add files into an indexed location on disk, copy or move them into that directory using your OS file manager (while Curio watches or rescans it), or use **Storage › Move to folder on disk** for existing assets.

## From browser

Drag a picture or link from a web browser onto Curio's window or a sidebar folder:

- **Windows**: If the browser hands over image bytes directly, Curio saves them via its temporary `dropped/` folder into managed storage without re-downloading. When only a URL is provided, Curio downloads the target or saves a web link via [Save web links and downloads](../import/links.md).
- **Linux**: Curio reads the dropped URL and downloads the asset or saves a web link.
- For browser capture with tags, folders, signed-in downloads and site rules, see [Capture from the browser](../import/browser.md).

## Within library

Dragging assets **inside** Curio depends on where the drag starts:

| Drag starts in | Drop target | Disk effect (`files/` hard links) | Library effect |
|---|---|---|---|
| **A library folder** | Another folder (sidebar, Pinned or subfolder card) | Managed file or location hard link moves into the target folder's directory | **Moves** assets out of the source folder (and shown subfolders) into the target folder |
| **All**, **Favorites**, **Tag**, **Smart folder** or a **Read-only** / **Edit files** location | A library folder | Creates a hard link (or copy across drives) in the target folder's directory; location original stays untouched | **Links** the asset into the target folder while keeping existing memberships |
| An indexed location set to **Edit and organize** | A library folder | After confirmation, moves files into `files/<folder>/` (across drives: verified copy, then original to OS Recycle Bin or Trash) | Converts assets to managed copies inside that folder |
| **Trash** | A library folder | Moves file out of `files/.trash/` into `<folder>/` | **Restores** assets into that target folder only |

## Modifiers

Hold a modifier key **before releasing** to override the default folder drop behavior:

| Modifier | Effect when dropping assets onto a folder |
|---|---|
| *(None)* | Default behavior above (Move from folder or **Edit and organize** location; Link from **All**, tags, smart folders and other locations). |
| ++shift++ | **Link (keep in source)**: Adds the assets to the target folder while keeping them in their current folder, and links **Edit and organize** location assets without moving their files. |
| ++ctrl++ | **Copy (duplicate)**: Creates independent copies with their own bytes on disk and copied metadata (tags, rating, notes and custom fields). Indexed location assets become new managed copies. |

### Between windows

- **Into another library's window**: Treated as a desktop copy drop. Files copy into the destination library along with their tags and notes.
- **Into a Curio plugin window**: Delivered to the plugin page via [`plugin.drop`](../../plugins/hooks/interaction.md#plugin-drop) with cursor coordinates. Dragging desktop files onto a plugin window imports unknown files first and hands the resulting assets to the page.

## Sidebar drops

- **Onto a tag** (sidebar or Tag Manager): Adds that tag to all dragged assets without touching files on disk (*Tag N items \<tag\>*).
- **Onto a Pinned folder**: Behaves identically to dropping onto that folder in the **Folders** tree.
- **Onto Trash**: When dragged from a folder while the asset belongs to other folders too, trashes only that folder's entry. Otherwise moves the whole asset to **Trash**.
- **Onto an indexed Location**: Locations in the sidebar do not accept asset drops (dragging a location row reorders the **Locations** list). Use **Storage › Move to folder on disk** to move assets into a writable location on disk.

## Drag out

Drag any selection out of Curio's grid onto your OS file manager, Photoshop, Blender, Figma, Discord or any desktop application:

- Curio starts a native OS file drag referencing the original file paths on disk.
- The receiving program decides whether to open, import or copy the files. Curio offers copy semantics so external apps do not delete managed files unexpectedly.
- If you drag an asset out over another window and back onto the same Curio window, dropping it on a folder links it into that folder. Dropping it back on the open grid does nothing.

## Drop preview

- While you drag inside Curio, an accent highlight marks valid drop targets. A floating pill states the exact operation and count (for example *Move 4 items from Inbox to References* or *Link 4 items into Favorites*).
- When a copy drop hits filename collisions in the target folder, Curio asks once for the whole drop: **Keep both**, **Replace** (replaces the existing file's bytes while keeping its tags, notes and folders; old bytes go to the OS Recycle Bin or Trash) or **Skip**.
- An asset in multiple folders has one hard link per folder under `<library>.curio/files/`. Editing, rotating or replacing the file updates every folder automatically without duplicating storage space.
