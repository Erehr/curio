# archive

Operations for listing and reading members inside ZIP, CBZ, EPUB, 7z and CB7 items, obtaining scaled entry URLs in plugin pages and packing existing items into new CBZ or ZIP archives. All operations target a **library**.

## archive.list {#archive-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/archive/list</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: archive_list</span>
</div>

List every entry inside a ZIP, CBZ, EPUB, 7z or CB7 item in natural reading order (folder by folder, `page2` before `page10`), including byte size, whether each entry is an image (`picture`), and image dimensions and animation flags. Reading the dimensions means reading the start of every image, so pass `sizes: false` when only the names, sizes and kinds matter, such as to count pages or find a cover.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Archive item UUID or row ID. |
| `sizes` | `boolean` | No | `true` | Whether image entries include `width`, `height` and `animated`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ entries: { name: string; size: number; picture: boolean; width?: number; height?: number; animated?: boolean }[] }`:

- `entries[].name` (`string`): Entry path inside the archive.
- `entries[].size` (`number`): Uncompressed entry size in bytes.
- `entries[].picture` (`boolean`): Whether the entry is an image file.
- `entries[].width` (`number`, optional): Display width in pixels for readable image entries (for 7z/CB7 once unpacked).
- `entries[].height` (`number`, optional): Display height in pixels for readable image entries.
- `entries[].animated` (`boolean`, optional): Whether the image entry is animated.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/archive/list \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-..."}'
    ```

=== "JavaScript"

    ```javascript
    const { entries } = await curio.archive.list({ id: comicId })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"archive_list","arguments":{"id":"0194b7a2-..."}}}
    ```

**Example response**

```json
{
  "entries": [
    { "name": "001_cover.jpg", "size": 412890, "picture": true, "width": 1600, "height": 2400, "animated": false },
    { "name": "ComicInfo.xml", "size": 1480, "picture": false }
  ]
}
```

**Events and errors** Read-only. Returns `400 bad_request` if the item is not a supported archive or cannot be opened, `403 not_allowed` without `files.read`, `404 not_found` if the item or its file is missing or offline, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## archive.entry {#archive-entry}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/archive/entry</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, Plugin (binary)</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Stream the raw bytes of a single archive entry. Over HTTP returns binary bytes; in `curio.js` resolves to a `Blob`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Archive item UUID or row ID. |
| `name` | `string` | Yes | — | Exact entry name from [`archive.list`](#archive-list). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** Binary response (`Blob` in `curio.js`).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/archive/entry \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"id":"0194b7a2-...","name":"ComicInfo.xml"}'
    ```

=== "JavaScript"

    ```javascript
    const blob = await curio.archive.entry({ id: comicId, name: 'ComicInfo.xml' })
    const xml = await blob.text()
    ```

**Example response** Raw entry bytes (`200 OK`).

**Events and errors** Read-only. Returns `400 bad_request` if the item is not a supported archive, `403 not_allowed` without `files.read`, `404 not_found` if the item, its file or the entry does not exist, `404 unknown_library`, `409 library_closed` / `library_offline` or `422 failed` on I/O error.

---

## archive.entryUrl {#archive-entryurl}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Obtain a scoped URL for a plugin page to display an archive entry directly (for example in `<img src>`), optionally downscaled by Curio to `size` pixels on its long edge.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Archive item UUID or row ID. |
| `name` | `string` | Yes | — | Exact entry name from `archive.list`. |
| `size` | `number` | No | Full size | Optional target long edge in pixels (`64`–`8192`) for image entries. |

**Returns** `{ url: string }`.

**Example request**

=== "Plugin page"

    ```javascript
    const { url } = await curio.archive.entryUrl({ id: comicId, name: '001_cover.jpg', size: 1200 })
    pageImg.src = url
    ```

**Example response**

```json
{ "url": "http://127.0.0.1:52190/plugin-archive/abc123...?size=1200" }
```

**Events and errors** Read-only. Returns `400 bad_request` if the item is not a supported archive, `403 not_allowed` outside plugin pages or without `files.read`; `404 not_found` if the item, its file or the entry does not exist.

---

## archive.create {#archive-create}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/archive/create</span>
  <span class="curio-badge">Permission: files.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: archive_create</span>
</div>

Pack existing library items into a new `.cbz` or `.zip` item in the exact order provided (7z is read-only).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | `string` | Yes | — | Output filename (`.cbz` appended automatically unless ending with `.cbz` or `.zip`). |
| `items` | `(string \| number)[]` | Yes | — | Ordered list of item IDs to pack into the archive. |
| `folders` | `number[]` | No | `[]` | Library folder IDs to place the new archive item in (first folder determines managed disk location). |
| `tags` | `string[]` | No | `[]` | Tag names to assign (created automatically if missing). |
| `annotation` | `string` | No | — | Notes for the new archive item. |
| `sourceUrl` | `string` | No | — | Source URL recorded on the new archive item. |
| `rating` | `number` | No | — | Star rating `0`–`5`. |
| `favorite` | `boolean` | No | — | Mark the new archive item as a favorite. |
| `explicit` | `boolean` | No | — | Mark the new archive item as explicit. |
| `fields` | `Record<string, unknown>` | No | — | Custom field values by key. |
| `requestId` | `string` | No | — | Idempotency token (max 128 characters, 10-minute window). |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Added`](../types/item.md#added).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/archive/create \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"name":"Chapter 01.cbz","items":["0194b7a2-...","0194b7a3-..."],"tags":["comics"]}'
    ```

=== "JavaScript"

    ```javascript
    const added = await curio.archive.create({
      name: 'Chapter 01.cbz',
      items: [page1, page2],
      tags: ['comics'],
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"archive_create","arguments":{"name":"Chapter 01.cbz","items":["0194b7a2-...","0194b7a3-..."]}}}
    ```

**Example response**

```json
{ "success": true, "ids": ["0194b7e0-..."], "errors": [] }
```

**Events and errors** Emits [`items.added`](../events/items.md#items-added). Returns `400 bad_request` if `items` or `name` is empty, `403 not_allowed` without `files.write`, `404 not_found` if any item, source file or folder ID is missing or offline, `404 unknown_library`, `409 library_closed` / `library_offline` or `422 failed` if archive creation fails.
