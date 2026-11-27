# duplicates

Operations for launching asynchronous duplicate searches across five comparison methods, resolving duplicate groups with metadata merging and safety rechecks and managing persistent ignore lists. All operations target a **library**.

## duplicates.find {#duplicates-find}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/duplicates/find</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: duplicates_find</span>
</div>

Start a background duplicate-detection job (canceling any duplicate scan already running on this library). Subscribe to [`job.finished`](../events/jobs.md#job-finished) or poll [`job.get`](job.md#job-get) to read the discovered groups once complete.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ways` | `string[]` | No | `["content"]` | Comparison modes: `content` (exact BLAKE3 bytes), `looks` (perceptual thumbnails), `video` (6 sampled frames), `sound` (first minute audio fingerprint), `archive` (ZIP/CBZ/EPUB/7z entry CRC-32s). |
| `thresholds` | `Record<string, number>` | No | `{ looks: 0.9, video: 0.9, sound: 0.7, archive: 0.9 }` | Per-mode similarity thresholds (`0.0`–`1.0`). |
| `lookKinds` | `string[]` | No | All except `video`/`audio` | Item kinds compared by `looks`. |
| `folder` | `number` | No | — | Limit scope to a folder and its subfolders. |
| `ids` | `(string \| number)[]` | No | — | Limit scope to specific items. |
| `query` | `string` | No | — | Limit scope to items matching this query. |
| `withLibrary` | `boolean` | No | `false` | Compare scoped items against the entire library instead of only against each other. |
| `minSize` | `number` | No | `0` | Skip files smaller than this many bytes. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ job: string }`. Once finished, `Job.result` holds the duplicate groups and omitted counts.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/duplicates/find \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ways":["content","looks"],"thresholds":{"looks":0.92}}'
    ```

=== "JavaScript"

    ```javascript
    const { job } = await curio.duplicates.find({
      ways: ['content', 'looks'],
      thresholds: { looks: 0.92 },
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"duplicates_find","arguments":{"ways":["content","looks"]}}}
    ```

**Example response**

```json
{ "job": "dup-8c21a4" }
```

**Events and errors** Emits [`job.started`](../events/jobs.md#job-started), [`job.progress`](../events/jobs.md#job-progress) and [`job.finished`](../events/jobs.md#job-finished). Returns `400 bad_request` for invalid `ways`, out-of-range `thresholds`, invalid `lookKinds`, invalid `query` syntax or `withLibrary: true` without a scope, `403 not_allowed` without `metadata.read`, `404 not_found` for an unknown `folder` or item ID, `404 unknown_library`, `409 library_closed` / `library_offline` or `422 failed` if starting the scan fails.

---

## duplicates.resolve {#duplicates-resolve}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/duplicates/resolve</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: duplicates_resolve</span>
</div>

Keep one item in each duplicate group and move the remaining copies to Trash after re-verifying that the kept file still exists and any `identical` copies still match byte-for-byte.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `groups` | `{ keep: string \| number; remove: (string \| number)[]; identical?: (string \| number)[] }[]` | Yes | — | Up to `1000` groups to resolve. IDs in `identical` are byte-rechecked against `keep` before trashing. |
| `combine` | `boolean` | No | `true` | Merge tags, folders, highest rating, notes, source URL and empty user custom fields from removed copies onto `keep`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ removed: number; kept: number; skipped: { id: string; reason: string }[] }`:

- `removed` (`number`): Total duplicate copies moved to Trash.
- `kept` (`number`): Number of groups resolved.
- `skipped` (`{ id: string; reason: string }[]`): Items or groups left untouched by safety rechecks (for example when the kept file is missing or an `identical` copy no longer matches byte-for-byte), with each item's UUID and reason.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/duplicates/resolve \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"combine":true,"groups":[{"keep":"0194b7a2-...","remove":["0194b7a9-..."],"identical":["0194b7a9-..."]}]}'
    ```

=== "JavaScript"

    ```javascript
    const res = await curio.duplicates.resolve({
      combine: true,
      groups: [{ keep: idA, remove: [idB], identical: [idB] }],
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"duplicates_resolve","arguments":{"groups":[{"keep":"0194b7a2-...","remove":["0194b7a9-..."]}]}}}
    ```

**Example response**

```json
{ "removed": 1, "kept": 1, "skipped": [] }
```

**Events and errors** Emits [`items.changed`](../events/items.md#items-changed) (on kept items) and [`items.removed`](../events/items.md#items-removed) (`purged: false`). Returns `400 bad_request` if more than 1,000 groups are given, `403 not_allowed` without `metadata.write`, `404 not_found` for an unknown item ID, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## duplicates.ignore {#duplicates-ignore}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/duplicates/ignore</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: duplicates_ignore</span>
</div>

Permanently mark all pairs among two or more items as **not duplicates** so future `duplicates.find` runs never group them together again.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | At least two item IDs. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ pairs: number }` — number of ignored pairs recorded.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/duplicates/ignore \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-...","0194b7a9-..."]}'
    ```

=== "JavaScript"

    ```javascript
    const { pairs } = await curio.duplicates.ignore({ ids: [idA, idB] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"duplicates_ignore","arguments":{"ids":["0194b7a2-...","0194b7a9-..."]}}}
    ```

**Example response**

```json
{ "pairs": 1 }
```

**Events and errors** Persists ignore pairs in SQLite. Returns `400 bad_request` if fewer than 2 items are given, `403 not_allowed` without `metadata.write`, `404 not_found` for an unknown item ID, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## duplicates.unignore {#duplicates-unignore}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/duplicates/unignore</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: duplicates_unignore</span>
</div>

Remove persistent duplicate-ignore marks for specific items, or clear all ignore marks in the library when `ids` is omitted.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | No | *(all ignored pairs)* | Clear marks involving these item IDs. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ pairs: number }` — number of ignored pairs removed.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/duplicates/unignore \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"ids":["0194b7a2-..."]}'
    ```

=== "JavaScript"

    ```javascript
    const { pairs } = await curio.duplicates.unignore({ ids: [idA] })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"duplicates_unignore","arguments":{}}}
    ```

**Example response**

```json
{ "pairs": 1 }
```

**Events and errors** Clears matching rows from the duplicate ignore table. Returns `403 not_allowed` without `metadata.write`, `404 not_found` for an unknown item ID, `404 unknown_library` or `409 library_closed` / `library_offline`.
