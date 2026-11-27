# Permissions

Curio separates **declarative contributions** (which need no permissions), **sandboxed HTML/JS pages** (per-plugin origins and CSP) and **native programs** (explicit `exec` trust and short-lived scoped run tokens).

## library.read

Every active plugin receives **`library.read`** and access to its own storage and frame helpers (`plugin.state.*`, `plugin.itemState.*`, `plugin.settings.*`, `plugin.log`, `plugin.close`, `viewer.setTools`, `ui.choose`, `ui.popover` and its own `job.*` lifecycle).

- **Library scoping**: A plugin is always scoped to the originating library of the window that launched it. It cannot pass another `library` or `window` target, and **`library.manage` is never grantable to any plugin**.
- **Declarative contributions**: `fields`, `png_text`/`regex`/`json` extractors, field `widgets`, `filters`, `sorts`, `panels`, `columns` and `theme_variables` require no permissions.

## metadata.*

| Permission | Grants | Without it |
|---|---|---|
| `metadata.read` | Query and read items, folders, smart folders, tags, tag groups, fields, locations, stats and duplicates. Program `stdin` includes full item metadata records. | Read calls return `403 not_allowed`. Program `stdin` carries only `uid` (plus `path` if `files.read` is granted). |
| `metadata.write` | Edit item names, notes, source URLs, ratings, favorite/explicit flags, tags, folders, grouping and custom fields via API or program stdout `set`. | Write calls return `403 not_allowed`. Program `set` replies fail and are logged. |

## organize

- **Library structure**: Grants creating, renaming, moving, styling and deleting folders, smart folders, tags, tag groups, locations and user-defined fields.
- **Field ownership**: Even with `metadata.write`, a plugin may only write fields **it owns** (or shares with the same type) and **user-defined fields**. It cannot write another plugin's fields or built-in reader fields. Keys starting with `__` are ignored except an extractor's `__title`, `__annotation` and `__source_url` (which only fill empty metadata).

## files.*

| Permission | Grants | Without it |
|---|---|---|
| `files.read` | Read file paths (`Item.path`), fetch bytes (`item.file`, `item.thumbnail`, `archive.entry`), get streaming URLs (`item.fileUrl`, `item.previewUrl`, `archive.entryUrl`) and inspect metadata (`item.exif`, `item.pngText`, `item.readFields`). Required for command/page extractors and thumbnailers. | `path` is omitted or `null`. Format thumbnailers and command/page extractors remain inactive. |
| `files.write` | Import via `item.add.*`, replace or rotate files, set thumbnails, purge Trash and upload via `item.writeUrl`. Also allows importing files from the plugin's own `process.run` temporary folder (`CURIO_RUN_FOLDER`). | Refused with `403 not_allowed`. |
| `files.outside` | Access disk paths outside the library and `CURIO_RUN_FOLDER` (`item.export`, `location.add`, plus `item.add.fromPaths`, `item.replaceFile`, `item.setThumbnail` and `item.add.beside` with external paths). | Refused with `403 not_allowed`. |

## ui

- **Window & dialogs**: Grants `selection.*`, `view.*`, `viewer.open`/`close`/`current`, `item.open`, `item.reveal`, `app.openUrl`, `ui.notify`/`confirm`/`prompt`/`pick*`/`menu`, `dialog.*` and `clipboard.*` on the originating library's window.
- **Unprivileged overlays**: `ui.choose` and `ui.popover` require no extra permission because they render anchored overlays for the plugin's own page.

## exec

- **Process execution**: Required to spawn `actions[].run`, `urls[].run`, `extractors[].command`, `formats[].thumbnail.command` and `process.run` (including declared external `tools[]`).
- **OS trust boundary**: Native child processes run with the user's operating system privileges. Curio's `files.*` and `network` permissions govern Curio's API and HTML pages, not OS syscalls inside a native executable. Granting `exec` trusts the plugin at the same level as any installed native program.
- **Run tokens**: Every spawned program receives `CURIO_API_URL` (pointing to the spawning process's private loopback endpoint, which refuses browser origins) and a single-run `CURIO_API_TOKEN` that is revoked when the process exits.

## network

- **Without `network`**: Plugin pages are restricted by Content Security Policy (`connect-src`) to their own plugin origin, `/curio/ui/`, `data:` and `blob:`.
- **With `network`**: Pages can connect only to the hostnames listed in `plugin.json`'s `network[]` array, or to any `https:` or `wss:` host when `network[]` is empty.

## Reapproval

Plugins bundled inside `<Library>.curio/plugins/` never run until approved on the current computer:

- **Content hash**: Approval computes a BLAKE3 hash over the plugin ID, folder path, granted permissions and every byte of every file inside the plugin directory (including `__pycache__`).
- **Python bytecode guard**: Python child processes run with `PYTHONDONTWRITEBYTECODE=1` so running a script does not create `.pyc` files and invalidate the folder's approval hash.
- **Invalidation**: Adding, editing or replacing any file inside a library-local plugin directory, or changing its granted permissions, invalidates approval and pauses the plugin until the user approves it again in **Settings › Plugins**.

## Refusals

- **Missing permissions**: Contributions lacking a required permission remain listed with a disabled reason (such as *"needs the 'exec' permission"*). API calls lacking a grant fail with `Curio.Error` (`code: "not_allowed"`).
- **Program `set` scope**: An action program's JSON `set` reply may only modify items passed in its `assets` input. URL handlers receive no initial items and may reference newly added items.
