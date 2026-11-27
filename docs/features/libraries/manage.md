# Manage

A Curio library is a self-contained folder ending in `.curio` that holds its SQLite index (`library.db`), thumbnails, backups and managed files. Keep one library for everything or maintain separate libraries for different projects, clients or media collections.

## Create & switch

When Curio opens with no library loaded, the Welcome screen offers two actions:

- **Create new library**: Opens the name dialog, then asks you to choose the parent folder for `<name>.curio`.
- **Open an existing library**: Opens a folder chooser for any `.curio` library folder. Curio checks for `library.db` inside it so ordinary folders are not opened as libraries by mistake.

You can also drop files, folders or an existing `.curio` folder onto the Welcome screen. Dropping a `.curio` folder opens it immediately. Dropping ordinary files or folders creates `My Library.curio` in Curio's default libraries folder and imports them.

Once a library is open, use the **Library switcher** at the top of the Library sidebar (or press ++ctrl+shift+o++) to switch libraries, create a new one or open an existing folder:

| Action | Where to find it | Result |
|---|---|---|
| Switch to a library | Click a library in the **Library switcher** or the **Libraries** sidebar section | Opens the library in the current window, or raises its window if it is already open elsewhere |
| Create a new library | **Library switcher › New library** (or **Welcome › Create new library**) | Prompts for a name and parent folder, creates `<name>.curio` and opens it |
| Open a library folder | **Library switcher › Open library folder** (or **Welcome › Open an existing library**) | Opens an existing `.curio` folder from anywhere on disk and adds it to your library list |
| Open in a second window | Library row **⋯ › Open in new window** | Launches a separate Curio window on that library while keeping the current window open |

!!! note "Safe library switching"
    When you switch libraries in the same window, Curio opens the new library before closing the old one. If opening the new library fails, your previous library stays open and active.

## Appearance

Every library you create or open is added to this computer's known library list. Adding, renaming, reordering, styling or removing a library syncs immediately across all open Curio windows.

- **Reorder libraries**: Drag any library row up or down in the **Library switcher** menu or the **Libraries** sidebar section.
- **Change icon and color**: Open the library's **⋯** menu (or right-click it in **Libraries**). Choose **Icon** to pick from the Tabler icon catalog, or click a color dot (`None`, `Accent`, `Red`, `Orange`, `Yellow`, `Green`, `Teal`, `Blue`, `Purple`, `Pink`).
- **Rename a library**: Open the library's **⋯** menu and choose **Rename**. Curio renames the folder on disk to `<new name>.curio` in the same parent folder and updates the remembered path. All assets, settings, folders and IDs stay intact.

!!! warning "Rename requires the library to be closed"
    A library cannot be renamed while any window has it open because its database and folder handles are locked. Switch to another library or open a second window first, then rename it from the switcher. If a background API process holds the library open, Curio asks it to release the library before renaming.

## Move

Because a `.curio` folder is self-contained, move it like any other folder on disk:

1. Close the library by switching to another library or quitting Curio.
2. Move the entire `<name>.curio` folder to its new path in your file manager.
3. Open Curio and choose **Library switcher › Open library folder**, or drop the moved `.curio` folder onto the window.
4. Remove the old entry from the library list with **⋯ › Remove from list**.

When Curio opens the moved library, it updates its internal managed-store path to the new `<name>.curio/files/` location. Indexed [locations](locations.md) keep pointing to their original disk paths and keep working unless those folders moved too.

!!! tip "Portable mode uses relative paths"
    In [portable mode](../../start/install.md), libraries on the same drive as `Curio data` are stored with relative paths. Moving the USB drive to another computer or drive letter keeps those libraries working automatically.

## Forget & reopen

To take a library off the switcher list without touching its files on disk, open its **⋯** menu and choose **Remove from list**. This option is available for closed libraries and for the library open in the current window.

- **Removing the active library**: Curio closes it, removes it from the remembered list and returns the window to the Welcome screen.
- **Library open in another window**: **Remove from list** is hidden until you close that window.
- **Files stay intact**: Removing a library from the list never deletes `<name>.curio` or any assets inside it.

To add a forgotten library back, choose **Open library folder** in the Library switcher, click **Open an existing library** on the Welcome screen or drop the `.curio` folder onto the Curio window.

## Default folder

When you create a library with **New library** or **Create new library**, Curio asks you to pick the parent folder for `<name>.curio`.

Curio also creates libraries automatically without asking, such as on a Welcome screen drop or a [`library.create`](../../api/operations/libraries.md) call without a `folder` parameter. Those libraries go in the default `Libraries` folder:

| Mode / OS | Default `Libraries` folder |
|---|---|
| Windows (installed) | `%APPDATA%\Curio\Libraries\` |
| Linux (installed) | `$XDG_DATA_HOME/Curio/Libraries/` (typically `~/.local/share/Curio/Libraries/`) |
| macOS | `~/Library/Application Support/Curio/Libraries/` |
| Portable mode (`Curio data` beside the executable) | `<program folder>/Curio data/Libraries/` |

View the open library's path, schema version and identifiers under **Settings › Storage › Library & thumbnail cache**.

## Related

- [Copied files and indexed locations](storage.md): What lives inside a `.curio` folder and how `files/` works.
- [Libraries in multiple windows](windows.md): Running libraries side by side, tray and menu bar behavior, and single-writer locking.
- [Libraries and statistics API](../../api/operations/libraries.md): Managing libraries programmatically (`library.create`, `library.rename`, `library.forget`, `library.setLook`).
