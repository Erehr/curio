# Folders

Folders organize assets into a nested hierarchy inside a library. An asset can belong to one folder, several folders or no folder. For managed assets copied into the library, Curio mirrors folders as real directories and hard links inside `<library>.curio/files/`. For assets linked in place from a location, the original file stays untouched in its location. When such an asset joins a library folder, Curio places a hard link (or a copy across drives) inside `<library>.curio/files/<folder>/`. See [Copied files and indexed locations](../libraries/storage.md) for how managed storage mirrors folders.

## Create

Create a top-level folder or subfolder from the Library sidebar, the grid background or the keyboard:

- Click **+** on the **Folders** heading, right-click a folder and choose **New folder**, or press ++ctrl+shift+n++ for a top-level folder.
- Right-click a folder and choose **New subfolder**, or press ++shift+alt+n++ inside a folder to create a subfolder.
- Right-click selected assets and choose **New folder from selection** to file them into a new folder right away.
- Right-click empty space in the grid and choose **New subfolder** inside a folder (or **New folder** outside one).

Type a name and press ++enter++ to save, or ++escape++ to cancel. Sibling folder names are compared case-insensitively because each folder maps to a directory in `files/`. If a sibling already uses that name, Curio appends a number using the style in **Settings › General › Background work › Duplicate names**.

!!! tip "Keyboard shortcuts for the folder tree"
    Press ++slash++ to expand or collapse all folders in the sidebar. You can also double-click any folder with subfolders, or right-click it and choose **Expand / collapse**, **Expand / collapse level** or **Expand / collapse all**.

## Membership

Curio folders are memberships, not exclusive containers. You can add an asset to more folders or move it out of its current folder:

| Method | Default effect | How to change or trigger |
|---|---|---|
| Folder picker (++f++ or ++ctrl+shift+d++) | Toggles membership in each checked folder | Check multiple folders to keep an asset in all of them; uncheck one to remove it without trashing |
| **Add to last folder** (++alt+d++) | Adds the selection to the most recently picked folder | Uses the library's recent folders list |
| Drag from **All**, a tag, search or smart folder onto a folder | Links the asset into the target folder | Hold ++ctrl++ to create independent file copies in that folder |
| Drag from a folder onto another folder | Moves the asset out of the source folder into the target folder | Hold ++shift++ to keep the asset in both folders; hold ++ctrl++ to copy |
| Inspector **Folders** widget | Lists every folder containing the asset as removable chips | Click **Add to folders** to open the picker, or remove a chip to take the asset out |

!!! warning "Removing from a folder vs. pressing Delete"
    - **Remove from folder** (via the folder picker, an Inspector chip or an action): Takes the asset out of that folder while keeping it in the library. If it belongs to no other folder and is not in a location, it appears in **Uncategorized**.
    - **Press ++delete++ inside a folder**: Moves that folder membership to **Trash** if the asset belongs to other folders too, or moves the entire asset to **Trash** if that is its only folder.

See [File operations and trash](file-actions.md) and [Drag and drop reference](drag-drop.md).

## Reorder

Folders keep a manual order at every level. Rearrange them by hand or sort them automatically:

- **Drag and drop**: Drag one or more selected folders in the sidebar. Drop near a sibling's top or bottom edge to place before or after it. Drop onto a folder to nest inside it, or drop onto the **Folders** heading to move to the top level.
- **Sort automatically**: Right-click any folder and choose an option under **Order folders**:

| Menu option | Scope | Order |
|---|---|---|
| **This level, A to Z** / **Z to A** | Siblings of the clicked folder | Name ascending or descending |
| **Everywhere, A to Z** / **Z to A** | Every level in the folder tree | Name ascending or descending |
| **This level, by count** / **by count ascending** | Siblings of the clicked folder | Total asset count (including subfolders), ties by name |
| **Everywhere, by count** | Every level in the folder tree | Total asset count descending, ties by name |
| **Subfolders, A to Z** / **Z to A** / **by count** | Direct children of the clicked folder (when it has subfolders) | Name or total asset count |

## Filter sidebar

The **Library filter** box at the bottom of the Library sidebar narrows folders, smart folders, tags and locations by fuzzy name matching. Press ++ctrl+alt+f++ to focus it, or to open its popover in compact sidebar mode.

Click the filter-options button inside **Library filter** to open folder scope controls:

- **Empty folders**: Cycles through three states: ignore emptiness, show only empty folders (count `0`) or show only non-empty folders.
- **Color**: Click one or more color swatches (including **None** for uncolored folders) to show only matching folders.

While a filter or scope option is active, matching folders and their parent branches expand automatically. Right-click or clear the filter-options button to reset the sidebar scope.

## Rename & delete

Select one folder (or multiple folders with ++ctrl++-click or ++shift++-click) to act on them from the context menu or keyboard:

- **Rename**: Right-click a folder and choose **Rename**, or select it and press ++f2++. Renaming is refused if a sibling already uses that name (case-insensitively).
- **Move**: Drag folders in the tree, or right-click and choose **Move to** or **Move to top**. Moving a folder into itself or its descendants is blocked. If a sibling with the same name exists at the destination, choose **Merge** (combining assets, trashed entries and matching subfolders), **Keep both** (numbering the moved folder) or **Cancel**.
- **Duplicate**: Right-click one or more folders and choose **Duplicate** to clone their folder structure.
- **Export or transfer**: Right-click folders to **Export files**, **Copy to another library** or **Move to another library** for all assets inside them.
- **Delete**: Right-click and choose **Remove folder**, or press ++delete++ with the sidebar focused. Empty folders with no subfolders delete immediately. Non-empty folders and folders with subfolders ask for confirmation first. Subfolders are deleted too. Assets in no other folder move to **Trash**, while assets in another folder stay in the library.

!!! note "Restoring assets from a deleted folder"
    Curio records the full folder path of every asset trashed when a folder is deleted. Restoring that asset from **Trash** recreates the deleted folder path and places the asset back inside it.

## Auto-tagging

A folder can attach tags automatically to every asset added to it:

1. Right-click the folder and choose **Auto-tag** (or use a custom **Folder auto-tags** shortcut in **Settings › Shortcuts**).
2. Click the tags box to pick existing tags, type new tag names or remove unwanted chips.
3. Edit the folder name if needed, then click **Save Changes**.

Saving auto-tags applies them immediately to every asset already in that folder and to any asset added later. Clearing a folder's auto-tags stops future tagging without removing tags from existing assets.

## Look & pinned

Customize folders so key branches stand out in the sidebar, toolbar menus and pickers:

- **Icon**: Right-click a folder and choose **Icon** to search the full Tabler icon set or reset to **Default icon**.
- **Color**: Right-click one or more folders and pick a swatch: **None**, **Accent**, **Red**, **Orange**, **Yellow**, **Green**, **Teal**, **Blue**, **Purple** or **Pink**. Folder colors also work with the **Folder color** filter and the sidebar color filter.
- **Pinned**: Right-click a folder, smart folder or location and choose **Pin** to add it to **Pinned** in the Library sidebar and **Pinned menu**. Right-click an entry and choose **Unpin** to remove it, or drag entries in **Pinned** to reorder them. Pinned folders are also drop targets for assets.
- **Recent folders**: Curio remembers the 20 most recently used folders per library. They power the **Recently used** tab in the folder picker, the 6 recent rows at the top of **All folders** and **Add to last folder** (++alt+d++).

!!! info "Pinned folders vs. favorite assets"
    The sidebar **Pinned** section holds shortcuts to folders, smart folders and locations. Individual assets use the **Favorite** heart flag in the Inspector **Rating** row, the asset context menu and the **Favorites** view.

## Picker & scope

Press ++f++ or ++ctrl+shift+d++ with assets selected (or click **Add to folders** in the Inspector or context menu) to open the folder picker:

- **Tabs**: Switch between **All folders** (up to 6 recent folders above the full tree), **Recently used** and **Selected** using the header buttons or ++tab++ / ++shift+tab++.
- **Search and create**: Type to fuzzy-search all folders (showing each match's parent name). If no folder matches exactly, click **Create "&lt;name&gt;"** or press ++enter++ to create it and file the selected assets immediately.
- **Keyboard navigation**: Press ++arrow-up++ and ++arrow-down++ to highlight a row, ++enter++ to toggle it and ++escape++ to close.
- **Subfolder scope**: A folder's sidebar count always includes distinct assets inside its subfolders. Toggle **Show subfolder contents** in the **Layout** popover, folder menu or grid menu to include subfolder assets in the grid. Toggle **Show subfolders** in the **Layout** popover to show or hide subfolder cards above the grid.

See [Grid, masonry, justified and list views](../browse/views.md) and the [Folder API reference](../../api/operations/folder.md).
