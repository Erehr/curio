# item.add

Operations for importing local files and directories, downloading remote URLs, uploading raw bytes, saving web bookmarks and adding converted/edited copies beside existing items. All operations target a **library**, require `files.write` and support optional `requestId` idempotency (10-minute deduplication window).

## item.add.fromPaths {#item-add-frompaths}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/add/fromPaths</span>
  <span class="curio-badge">Permission: files.write (+ files.outside unless plugin run folder)</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_add_fromPaths</span>
</div>

Import files or directories from the local computer. Files outside existing locations are copied into managed storage (`files/`); directories are added as in-place locations unless `copy: true` copies them into managed folders. Files created inside a plugin's own `CURIO_RUN_FOLDER` do not require `files.outside`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `paths` | `string[]` | Yes | — | Non-empty list of absolute file or folder paths on disk. |
| `copy` | `boolean` | No | `false` | Copy directories into managed storage as library folders instead of adding them as locations. |
| `move` | `boolean` | No | `false` | Move files into managed storage instead of copying (files only, not directories). |
| `onClash` | `string` | No | `"keep"` | Filename collision policy in destination folder: `keep` (number suffix), `replace` or `skip`. |
| `wait` | `boolean` | No | `false` | Wait up to 120 s and return created `ids` instead of returning `{ queued: true }` immediately. |
| `folders` | `number[]` | No | `[]` | Library folder IDs to place new items in (first folder determines managed disk location). |
| `tags` | `string[]` | No | `[]` | Tag names to assign (created automatically if missing). |
| `annotation` | `string` | No | — | Notes for the new items. |
| `sourceUrl` | `string` | No | — | Source URL recorded on new items. |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Mark the new items as favorites. |
| `explicit` | `boolean` | No | — | Mark the new items as explicit. |
| `fields` | `Record<string, unknown>` | No | — | Custom field values by key. |
| `requestId` | `string` | No | — | Idempotency token (max 128 chars, 10-minute window). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** `{ success: true; queued: true } | { success: true; ids: string[] }` (with optional `repeat: true`).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/add/fromPaths \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"paths":["C:\\Imports\\concept.png"],"folders":[3],"tags":["concept"],"wait":true}'
    ```

=== "JavaScript"

    ```javascript
    const res = await curio.item.add.fromPaths({
      paths: ['C:\\Imports\\concept.png'],
      folders: [3],
      tags: ['concept'],
      wait: true,
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"item_add_fromPaths","arguments":{"paths":["C:\\Imports\\concept.png"],"tags":["concept"],"wait":true}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7c0-1a2b-7c3d-8e4f-5a6b7c8d9e0f"] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added) (with `requestId` if supplied). Returns `400 bad_request` for empty `paths`, `move: true` on a directory or unknown `onClash`; `403 not_allowed` without `files.outside` for external paths; `404 not_found` for unknown folder IDs.

---

## item.add.fromUrls {#item-add-fromurls}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/add/fromUrls</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_add_fromUrls</span>
</div>

Download remote images or files into managed storage, or save web pages as `.url` links. Curio only downloads from the public internet (refusing loopback and private network addresses) and dispatches matching enabled plugin URL handlers first unless `handlers: false`. To save a single web address directly as a bookmark without URL handler dispatch or duplicate checks, see [`item.add.bookmark`](#item-add-bookmark).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `urls` | `string[]` | Yes | — | Public `http://` or `https://` URLs. |
| `name` | `string` | No | Derived from URL/response | Explicit display name (when passing a single URL). |
| `as` | `string` | No | `"auto"` | `auto` (download files, save pages as links), `file` (always download) or `link` (always save bookmark). |
| `alternates` | `string[]` | No | `[]` | Higher-resolution/alternate URLs tried in order before `urls[0]` (single-URL calls only). |
| `referer` | `string` | No | — | `Referer` header (`http`/`https` only). |
| `userAgent` | `string` | No | Curio default | Custom `User-Agent` header. |
| `cookies` | `string` | No | — | Per-request `Cookie` header sent strictly to the URL's own host (stripped on cross-host redirects, never stored; disallowed for plugins). |
| `handlers` | `boolean` | No | `true` | Allow matching plugin URL handlers to claim matching URLs (`false` bypasses them). |
| `onDuplicate` | `string` | No | `"keep"` | Byte/URL duplicate policy: `keep`, `skip` or `tell` (returns `duplicates` list). |
| `wait` | `boolean` | No | `true` | Wait up to 60 s for results (`false` queues immediately and returns `{ queued: true }`). |
| `folders` | `number[]` | No | `[]` | Library folder IDs to place new items in (first folder determines managed disk location). |
| `tags` | `string[]` | No | `[]` | Tag names to assign (created automatically if missing). |
| `annotation` | `string` | No | — | Notes for the new items. |
| `sourceUrl` | `string` | No | — | Source URL recorded on new items (overrides the downloaded URL when set). |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Mark the new items as favorites. |
| `explicit` | `boolean` | No | — | Mark the new items as explicit. |
| `fields` | `Record<string, unknown>` | No | — | Custom field values by key. |
| `requestId` | `string` | No | — | Idempotency token (max 128 chars, 10-minute window). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Added`](../types/item.md#added).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/add/fromUrls \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"urls":["https://example.com/art.png"],"name":"Sunset Study","tags":["web"],"onDuplicate":"skip","wait":true}'
    ```

=== "JavaScript"

    ```javascript
    const added = await curio.item.add.fromUrls({
      urls: ['https://example.com/art.png'],
      name: 'Sunset Study',
      tags: ['web'],
      onDuplicate: 'skip',
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"item_add_fromUrls","arguments":{"urls":["https://example.com/art.png"],"tags":["web"]}}}
    ```

**Example response**

```json
{
  "success": true,
  "ids": ["0194b7c1-2b3c-7d4e-9f0a-1b2c3d4e5f60"],
  "errors": []
}
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added). Individual download failures appear in `errors[]` rather than aborting the whole batch.

---

## item.add.fromBytes {#item-add-frombytes}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/add/fromBytes</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_add_fromBytes</span>
</div>

Import a file from inline base64 data (up to 64 MiB HTTP body) or from a previously spooled [`POST /api/v1/upload`](../connect/files.md#uploads) ticket ID (up to 2 GiB).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Filename with extension (e.g. `screenshot.png`). |
| `data` | `string` | One of `data` / `upload` | — | Raw base64 string or `data:<mime>;base64,...` URI. |
| `upload` | `string` | One of `data` / `upload` | — | 32-hex upload ID returned by `POST /api/v1/upload` (web tokens only, single-use, 10-minute expiry). |
| `url` | `string` | No | — | Source URL recorded on the item (overridden by `sourceUrl` if both are given). |
| `onDuplicate` | `string` | No | `"keep"` | `keep`, `skip` or `tell`. |
| `folders` | `number[]` | No | `[]` | Library folder IDs to place the new item in (first folder determines managed disk location). |
| `tags` | `string[]` | No | `[]` | Tag names to assign (created automatically if missing). |
| `annotation` | `string` | No | — | Notes for the new item. |
| `sourceUrl` | `string` | No | — | Source URL recorded on the new item (takes precedence over `url` if both are given). |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Mark the new item as a favorite. |
| `explicit` | `boolean` | No | — | Mark the new item as explicit. |
| `fields` | `Record<string, unknown>` | No | — | Custom field values by key. |
| `requestId` | `string` | No | — | Idempotency token (max 128 chars, 10-minute window). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Added`](../types/item.md#added).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/add/fromBytes \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"name":"pixel.png","data":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==","tags":["test"]}'
    ```

=== "JavaScript"

    ```javascript
    const added = await curio.item.add.fromBytes({
      name: 'pixel.png',
      data: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
      tags: ['test'],
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"item_add_fromBytes","arguments":{"name":"note.txt","data":"SGVsbG8gQ3VyaW8K"}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7c2-3c4d-7e5f-8011-223344556677"], "errors": [] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added). Returns `400 bad_request` on invalid base64 or expired/missing upload ID.

---

## item.add.bookmark {#item-add-bookmark}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/add/bookmark</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_add_bookmark</span>
</div>

Save any web address directly as a `.url` link item without downloading the target resource. For batch URL saving with plugin URL handlers or `onDuplicate` checks, use [`item.add.fromUrls`](#item-add-fromurls) with `as: "link"`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `url` | `string` | Yes | — | Target web URL saved in the `.url` link and recorded as the item's source URL. |
| `name` | `string` | No | `url` | Bookmark title. |
| `folders` | `number[]` | No | `[]` | Library folder IDs to place the new item in (first folder determines managed disk location). |
| `tags` | `string[]` | No | `[]` | Tag names to assign (created automatically if missing). |
| `annotation` | `string` | No | — | Notes for the new item. |
| `sourceUrl` | `string` | No | — | Optional source URL metadata override (defaults to `url` if omitted; does not change the `.url` link target). |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Mark the new item as a favorite. |
| `explicit` | `boolean` | No | — | Mark the new item as explicit. |
| `fields` | `Record<string, unknown>` | No | — | Custom field values by key. |
| `requestId` | `string` | No | — | Idempotency token (max 128 chars, 10-minute window). |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Added`](../types/item.md#added).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/add/bookmark \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"url":"https://zensical.org/","name":"Zensical Docs","tags":["docs"]}'
    ```

=== "JavaScript"

    ```javascript
    const added = await curio.item.add.bookmark({
      url: 'https://zensical.org/',
      name: 'Zensical Docs',
      tags: ['docs'],
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"item_add_bookmark","arguments":{"url":"https://zensical.org/","name":"Zensical Docs"}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7c3-4d5e-7f60-9122-334455667788"], "errors": [] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added).

---

## item.add.beside {#item-add-beside}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/item/add/beside</span>
  <span class="curio-badge">Permission: files.write (+ files.outside for external path)</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: item_add_beside</span>
</div>

Create a new item beside an existing item, copying its folder memberships, tags, rating, notes, source URL and custom fields.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Existing item UUID or row ID to inherit metadata and folders from. |
| `path` | `string` | One of `path` / `data` | — | File path on disk (requires `files.outside` unless created in plugin's run folder). |
| `data` | `string` | One of `path` / `data` | — | File bytes as base64 or `data:` URI. |
| `name` | `string` | No | Source item name + new extension | Filename with extension for the new item. |
| `library` | `string` | No | Default library | Routing parameter: library ID from [`library.list`](libraries.md#library-list), folder path or name. Not allowed for plugin callers. |

**Returns** [`Added`](../types/item.md#added).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/item/add/beside \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012","path":"C:\\Temp\\dawn_clouds.webp","name":"dawn_clouds.webp"}'
    ```

=== "JavaScript"

    ```javascript
    const added = await curio.item.add.beside({
      id: item.id,
      data: webpDataUrl,
      name: 'dawn_clouds.webp',
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"item_add_beside","arguments":{"id":"0194b7a2-...","path":"C:\\Temp\\dawn_clouds.webp"}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7c4-5e6f-7071-a233-445566778899"], "errors": [] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added). For large plugin-page outputs without base64 overhead, use [`item.writeUrl`](item-files.md#item-writeurl) with `mode: "beside"`.
