# Authorization

Curio checks a declared permission before running any API operation. External scripts, browser extensions and MCP clients authenticate with bearer tokens created in **Settings › API** (or through extension pairing). Plugins receive an automatic identity scoped to their approved manifest permissions and originating library.

## Create token

To create a token manually:

1. Open **Settings › API**.
2. Click **New token** under **Tokens**.
3. Enter a name (defaults to `Token`), choose permissions or a preset and optionally restrict libraries and origins.
4. Click **Make token** and copy the secret right away.

Each token secret uses the format `curio_<64 hex characters>` (two UUIDv4 hex strings with a `curio_` prefix). Curio shows the plaintext secret once at creation. It saves only the BLAKE3 hex hash alongside the token's metadata (`id`, `name`, `perms`, `origins`, `libraries`, `created` and `paired`) in this computer's `app.json`. Curio never writes tokens inside a `.curio` library folder.

!!! warning "Lost secrets cannot be recovered"
    Because Curio stores only the BLAKE3 hash, you cannot view a token secret again after closing the creation banner. If you lose a secret, delete the token and create a new one.

## Permissions

Curio defines **11 permissions**. Web and MCP tokens can hold **9 permissions**; `exec` and `network` apply only to plugins.

| Permission | Available to | What it allows |
|---|---|---|
| `library.read` | Tokens, Plugins | Read [`app.info`](../operations/app.md#app-info), [`app.operations`](../operations/app.md#app-operations), [`app.fonts`](../operations/app.md#app-fonts), [`app.theme`](../operations/app.md#app-theme), [`library.list`](../operations/libraries.md#library-list), [`library.info`](../operations/libraries.md#library-info), [`job.list`](../operations/job.md#job-list), [`job.get`](../operations/job.md#job-get) and [`tool.list`](../operations/tool.md#tool-list); subscribe to app, library, scan and job events; delete the calling token via [`token.forget`](../operations/credentials.md#token-forget). Every active plugin receives this permission automatically. |
| `metadata.read` | Tokens, Plugins | Query, count and inspect items, folders, smart folders, tags, tag groups, custom fields, locations, facets, library statistics, similarity and duplicates; subscribe to item and organization events. |
| `metadata.write` | Tokens, Plugins | Edit item names, notes, URLs, ratings, favorites, explicit flags, tags, folders, custom fields, display rotation and grouping; trash and restore items; refresh thumbnails and palettes; clean up or rescan fields; resolve or ignore duplicates. |
| `organize` | Tokens, Plugins | Create, rename, move, update and delete folders, smart folders, tags and tag groups; define and delete custom fields; manage location access, appearance, rescans, relocation and removal. |
| `files.read` | Tokens, Plugins | Receive absolute filesystem `path` fields in responses (omitted without this permission); read item files, thumbnails, EXIF, PNG text chunks, archive contents and plugin page URLs. |
| `files.write` | Tokens, Plugins | Upload files via [`POST /api/v1/upload`](files.md#uploads); add items from paths, URLs, bytes, bookmarks or beside existing items; create archives; rotate or replace files; set custom thumbnails; duplicate, copy or move items between libraries; purge trashed items; forget grouped files. |
| `files.outside` | Tokens, Plugins | Access disk paths outside the library store: export items ([`item.export`](../operations/item-lifecycle.md#item-export)), index a folder in place ([`location.add`](../operations/location.md#location-add)) and pass `path` arguments to [`item.add.fromPaths`](../operations/item-add.md#item-add-frompaths), [`item.add.beside`](../operations/item-add.md#item-add-beside), [`item.replaceFile`](../operations/item-files.md#item-replacefile) or [`item.setThumbnail`](../operations/item-files.md#item-setthumbnail). |
| `library.manage` | Tokens only | Open, create, rename, forget, reorder and style libraries on this computer; switch which library a window displays ([`library.switch`](../operations/libraries.md#library-switch)). Never granted to plugins. |
| `ui` | Tokens, Plugins | Read and change window selection, scope, search, filters, font sample text and viewer state; open or reveal items; open external URLs; show notifications, prompts, confirmations, pickers, menus and file dialogs; read or write the clipboard; subscribe to window UI events. |
| `exec` | Plugins only | Run bundled plugin programs or declared tools via [`process.run`](../operations/process.md#process-run) and view resolved executable paths in [`tool.list`](../operations/tool.md#tool-list). |
| `network` | Plugins only | Allow a plugin page's Content Security Policy to reach hosts listed in the plugin manifest's `network` field. |

**Settings › API presets:**

| Preset | Permissions granted |
|---|---|
| Read only | `library.read`, `metadata.read` |
| Read and add | `library.read`, `metadata.read`, `metadata.write`, `files.write` |
| Everything | All 9 token permissions (`library.read`, `metadata.read`, `metadata.write`, `organize`, `files.read`, `files.write`, `files.outside`, `library.manage`, `ui`) |

Calling an operation without its required permission returns HTTP `403` (`not_allowed`) and names the missing permission. Some operations also check argument-dependent permissions. For example, [`item.add.fromPaths`](../operations/item-add.md#item-add-frompaths) requires `files.write` plus `files.outside`, unless the paths come from the calling plugin's own `process.run` working folder.

## Scope & origins

You can narrow any token in **Settings › API** along two extra dimensions:

- **Allowed libraries (`libraries`)**: A list of stable library IDs.
  - When empty, the token can reach every library on this computer.
  - When non-empty, [`library.list`](../operations/libraries.md#library-list) and [`library.listChanged`](../events/libraries.md#library-listchanged) include only those libraries. Curio also filters event streams to those libraries and rejects calls targeting unlisted libraries with HTTP `403` (`not_allowed`).
- **Allowed browser origins (`origins`)**: A space-separated list of origins such as `chrome-extension://chfhdgecmnomfokannimcbbjgghdmejg` or `https://app.example.com`, stored lowercase without trailing slashes.
  - Requests with no `Origin` header (scripts, CLI tools, Node.js, Python and MCP clients) always pass.
  - Requests from loopback origins (`http://` or `https://` on `127.0.0.1`, `localhost` or `[::1]` on any port) always pass.
  - Requests from other origins pass when `origins` is empty or includes the normalized `Origin`. Otherwise Curio returns HTTP `403` (`not_allowed`).

## Extension limits

Tokens created through browser pairing ([`POST /api/v1/pair`](launch-pair.md#post-apiv1pair)) carry `paired: true`. Curio pins each paired token to the extension's exact `chrome-extension://` or `moz-extension://` origin and scopes it to the libraries chosen in the approval dialog. Paired tokens start with 5 permissions:

- `library.read`
- `metadata.read`
- `metadata.write`
- `organize`
- `files.write`

Even though paired tokens hold `files.write`, Curio enforces a hard deny list on every `paired: true` token. A paired extension can never delete files permanently, overwrite files in place or attach files beside existing items:

| Refused operation | Why paired tokens cannot call it |
|---|---|
| [`item.add.beside`](../operations/item-add.md#item-add-beside) | Attaches a new file beside an existing item instead of importing a standalone web capture. |
| [`item.purge`](../operations/item-lifecycle.md#item-purge) | Permanently deletes trashed item files from the library. |
| [`item.forgetFile`](../operations/item-grouping.md#item-forgetfile) | Removes a grouped member file record from an item and the library. |
| [`item.replaceFile`](../operations/item-files.md#item-replacefile) | Overwrites an existing item's file on disk. |
| [`item.rotateFile`](../operations/item-files.md#item-rotatefile) | Rewrites image bytes directly on disk. |
| [`item.moveToLibrary`](../operations/item-lifecycle.md#item-movetolibrary) | Removes items from the source library after copying them elsewhere. |

Paired tokens also omit `files.outside`, `files.read`, `library.manage` and `ui`. Operations such as [`item.export`](../operations/item-lifecycle.md#item-export), [`item.add.fromPaths`](../operations/item-add.md#item-add-frompaths), [`location.add`](../operations/location.md#location-add) and window automation calls remain blocked unless the user adds those permissions in **Settings › API**. Editing a paired token in Settings never removes the 6-operation deny list above.

## Plugin tokens

Plugins never use web tokens from **Settings › API**. Curio authenticates each plugin under its own plugin identity:

- **Implicit `library.read`**: Every active plugin receives `library.read` plus the permissions approved for that plugin in the current library. Plugins can never hold `library.manage`.
- **Per-plugin isolation**: Curio partitions plugin state ([`plugin.state.*`](../operations/plugin.md#plugin-state-get)), item state ([`plugin.itemState.*`](../operations/plugin.md#plugin-itemstate-get)), settings ([`plugin.settings.*`](../operations/plugin.md#plugin-settings-get)), logs ([`plugin.log`](../operations/plugin.md#plugin-log)), jobs ([`job.*`](../operations/job.md#job-list)) and `job.*` events by plugin ID. Each plugin sees only its own data and jobs.
- **Plugin pages**: HTML pages inside sandboxed iframes call the API through the window's `postMessage` bridge (`Curio.connect()` with no token). They cannot pass `library` or `window` parameters to reach another library or window.
- **Plugin programs and run tokens**: For native plugin programs (actions, URL handlers, extractors, thumbnailers or [`process.run`](../operations/process.md#process-run)), Curio starts an ephemeral loopback HTTP server. It injects four environment variables:
  - `CURIO_API_URL`: `http://127.0.0.1:<ephemeral-port>/api/v1/`
  - `CURIO_API_TOKEN`: `curio_run_<64 hex characters>`
  - `CURIO_PLUGIN`: The plugin's manifest ID.
  - `PYTHONDONTWRITEBYTECODE=1`: Stops Python bytecode writes from changing approved plugin hashes.

Run tokens stay only in memory and reject any request carrying an `Origin` header. Curio re-checks the plugin's live approved permissions on every call, so revoking a permission or disabling the plugin takes effect immediately. Run tokens expire as soon as the program exits.

## Revoke

- **Edit or narrow a token**: Edit any token in **Settings › API** to rename it or change its permissions, libraries and origins. Curio saves changes to `app.json` immediately without rotating the secret.
- **Active SSE stream enforcement**: Curio checks every open [`POST /api/v1/events`](events.md) stream against its token record every **3 seconds**. If you delete or edit the token (permissions, libraries, origins or name), Curio closes the stream so the client reconnects under the updated grant.
- **Revoke a token**: Delete the token in **Settings › API**. Future HTTP and MCP requests using that secret fail immediately with HTTP `401` (`unauthorized`).
- **Self-revocation (`token.forget`)**: Any web token can delete itself by calling [`token.forget`](../operations/credentials.md#token-forget) (requires `library.read`; available to web tokens, not MCP or plugins). Browser extensions call `token.forget` when the user clicks **Disconnect**.
