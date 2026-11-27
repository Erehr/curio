# Files

Add assets to a library through file dialogs, drag and drop, clipboard paste, watched folders, browser capture or the API. Every method keeps original file contents unchanged and queues thumbnails, color palettes and enabled metadata readers in the background.

## File chooser

Click the **+** (**Add**) button in the sidebar header to pick files or folders from disk:

- **Import files**: Pick one or more files from a native file dialog. Curio copies them into managed storage (`<Library>.curio/files/`) and places them in the open folder, or leaves them unfiled in **All**.
- **Import folder**: Pick a directory on disk to copy into managed storage, mirroring its subfolder tree as Curio folders.
- **Add location**: Pick a directory on disk to index in place without copying files. See [Add and manage locations](../libraries/locations.md).

!!! tip "Auto-tags on the target folder"
    When you import directly into a library folder that has **Auto-tag** configured, Curio applies those tags to every new asset. See [Folders and pinned items](../organize/folders.md).

## Drag & drop

Drag files or folders from Windows Explorer, Finder, a Linux file manager or another app onto Curio:

| Drop target | Result |
|---|---|
| **Main grid** (outside a folder) | Copies files into managed storage (`<Library>.curio/files/`). Dropped directories become nested Curio folders. Use **Add location** to index a directory in place. |
| **A library folder** (sidebar or subfolder card) | Copies files into managed storage, files them into that folder and applies its auto-tags. |
| **Files already inside an indexed Location** | Links the existing location assets without making duplicate copies, and adds them to the target folder if dropped onto one. |
| **A `.curio` library folder** | Opens that library instead of importing its files. |
| **An Eagle `.library` folder** | Imports the Eagle library into managed storage with its folders, tags and supported smart folders (see [Bring an Eagle library into Curio](eagle.md)). |

Drops never overwrite or move files into an indexed location directory on disk. Use **Storage › Move to folder on disk** for disk moves into writable locations. See [Drag and drop reference](../organize/drag-drop.md) for modifier keys, drop preview labels and cross-window drag rules.

## Folder trees

When you copy a directory tree into Curio through **Import folder**, drag and drop or the API:

- Each subdirectory becomes a matching Curio folder nested under the drop target, or at the top level of **Folders**.
- Existing folders with the same name at that level are reused instead of duplicated.
- Inside `<Library>.curio/files/`, managed files mirror that folder hierarchy so browsing `files/` in a file manager shows the same tree.

## Clipboard paste

Press ++ctrl+v++ anywhere in the main window (outside text inputs) to import from your clipboard:

- **Copied files**: Files copied in Explorer, Finder or a Linux file manager are imported into the active folder or **All**.
- **Clipboard image**: Screenshots or copied image data (such as from Snipping Tool or an image editor) are saved as a timestamped PNG file in managed storage.
- **Plain-text file path**: A copied local file path is resolved and imported.
- **Web URL**: An `http://` or `https://` address runs the [Save web links and downloads](links.md) pipeline. Curio checks plugin URL handlers first, then downloads files or saves web pages as bookmarks. To import several URLs at once, use **+ › Import from links**.

## Duplicates

- **Files already inside an indexed location**: Dropping or adding a file whose path already belongs to one of the library's locations does not create a managed copy. Curio links the existing location asset and adds it to the target folder if dropped onto one.
- **Name collisions in a folder**: When a copied file shares a name with an existing asset in the target folder, Curio prompts once for the batch: **Keep both** (adds a numeric suffix using your library's style), **Replace** or **Skip**.
- **Duplicate detection**: Content hashing runs in the background. Review exact byte duplicates or similar assets anytime with [Find duplicates](../tools/duplicates.md).

## Progress

Large imports run on the library's background worker so you can keep browsing while files copy and index:

- **Progress toasts**: Active imports and background jobs appear as persistent toasts with a progress bar and a **Cancel** button. Up to three non-error toasts stack at once.
- **Notifications & sound effects**: Configure when system notifications appear (**Always**, **In background**, **Failures only** or **Off**) and toggle sound effects for **Imports & downloads** under **Settings › Notifications**.
- **Post-import workers**: Once files enter the database, Curio generates thumbnails, extracts dominant color palettes, checks animation flags and runs enabled metadata readers. See [Indexing, previews and performance](../help/performance.md).
