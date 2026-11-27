# view

Operations for inspecting and driving what an open Curio window shows: its sidebar scope, search bar text, font sample text, Filter bar chips and single-filter navigation. All six operations target a **window** (accepting optional `window` or `library`), require the `ui` permission and return the updated [`View`](../types/runtime.md#view).

## view.get {#view-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/get</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_get</span>
</div>

Return the active scope, search text, effective combined query, item count, workspace mode and font sample text of a window.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/get \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const view = await curio.view.get()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"view_get","arguments":{}}}
    ```

**Example response**

```json
{
  "show": "folder",
  "id": 3,
  "path": null,
  "name": "References",
  "search": "",
  "query": "rating >= 4",
  "count": 48,
  "view": "grid",
  "sample": { "text": "" }
}
```

**Events and errors** Read-only. Returns `403 not_allowed` without `ui`, `404 not_found` for an unknown `window`, `404 unknown_library`, `409 no_window` if no window is open or `503 busy` if the window does not answer within 15 seconds.

---

## view.show {#view-show}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/show</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_show</span>
</div>

Navigate the target window to a quick view, folder, smart folder, tag or location. Clears active search and Filter bar chips unless the window's **Filter lock** is enabled.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `what` | `string` | Yes | — | `all`, `favorites`, `uncategorized`, `untagged`, `recent`, `random`, `trash`, `missing`, `folder`, `smartFolder`, `tag` or `location`. |
| `id` | `string \| number` | For `folder`, `smartFolder`, `tag`, `location` | — | Numeric ID for folder/smartFolder/location; name or numeric ID for `tag`. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/show \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"what":"folder","id":3}'
    ```

=== "JavaScript"

    ```javascript
    const view = await curio.view.show({ what: 'tag', id: 'sky' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"view_show","arguments":{"what":"folder","id":3}}}
    ```

**Example response**

```json
{ "show": "folder", "id": 3, "path": null, "name": "References", "search": "", "query": "", "count": 120, "view": "grid", "sample": { "text": "" } }
```

**Events and errors** Emits [`view.changed`](../events/windows.md#view-changed). Returns `400 bad_request` for unknown `what` or missing `id`, `403 not_allowed` without `ui`, `404 not_found` if target folder/smartFolder/tag/location or `window` does not exist, `404 unknown_library`, `409 no_window` or `503 busy`.

---

## view.setSearch {#view-setsearch}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/setSearch</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_setSearch</span>
</div>

Set the text in the target window's search box (plain words or [Query language](../query/syntax.md)). Pass `""` to clear.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `text` | `string` | Yes | — | Search text or query expression (max 10,000 characters; empty string clears). |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/setSearch \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"text":"rating >= 4 AND tag = landscape"}'
    ```

=== "JavaScript"

    ```javascript
    const view = await curio.view.setSearch({ text: 'rating >= 4 AND tag = landscape' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"view_setSearch","arguments":{"text":"rating >= 4"}}}
    ```

**Example response**

```json
{ "show": "all", "id": null, "path": null, "name": "All", "search": "rating >= 4 AND tag = landscape", "query": "rating >= 4 AND tag = landscape", "count": 29, "view": "grid", "sample": { "text": "" } }
```

**Events and errors** Emits [`view.changed`](../events/windows.md#view-changed). Returns `400 bad_request` if `text` exceeds 10,000 characters or has invalid query syntax, `403 not_allowed` without `ui`, `404 not_found` for an unknown `window`, `404 unknown_library`, `409 no_window` or `503 busy`.

---

## view.setSample {#view-setsample}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/setSample</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_setSample</span>
</div>

Set the shared sample string rendered by the List view's **Font sample** column and the **Fonts** viewer. Pass `""` to restore each font's own default script sample.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `text` | `string` | No | Unchanged | Sample text (max 1,000 characters; `""` resets to per-font default). |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/setSample \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"text":"Sphinx of black quartz, judge my vow"}'
    ```

=== "JavaScript"

    ```javascript
    await curio.view.setSample({ text: 'Sphinx of black quartz, judge my vow' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"view_setSample","arguments":{"text":"The quick brown fox"}}}
    ```

**Example response**

```json
{ "show": "all", "id": null, "path": null, "name": "All", "search": "", "query": "", "count": 420, "view": "grid", "sample": { "text": "Sphinx of black quartz, judge my vow" } }
```

**Events and errors** Emits [`view.changed`](../events/windows.md#view-changed). Returns `400 bad_request` if `text` exceeds 1,000 characters, `403 not_allowed` without `ui`, `404 not_found` for an unknown `window`, `404 unknown_library`, `409 no_window` or `503 busy`.

---

## view.setFilters {#view-setfilters}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/setFilters</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_setFilters</span>
</div>

Replace the active Filter bar selections in the target window. Any filter omitted from the request is cleared (passing `{}` clears all filters).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tags` | `(string \| number)[]` | No | `[]` | Include tags by name or ID. |
| `tagRule` | `string` | No | `"any"` | `any`, `all` or `exactly`. |
| `tagsNot` | `(string \| number)[]` | No | `[]` | Exclude tags by name or ID. |
| `untagged` | `boolean` | No | `false` | Include untagged items. |
| `folders` | `number[]` | No | `[]` | Include folder IDs. |
| `foldersNot` | `number[]` | No | `[]` | Exclude folder IDs. |
| `ratings` | `number[]` | No | `[]` | Star ratings `0`–`5`. |
| `favorite` | `boolean` | No | — | `true` for favorites only, `false` to exclude favorites. |
| `explicit` | `boolean` | No | — | `true` for explicit items only, `false` to exclude explicit items. |
| `kinds` | `string[]` | No | `[]` | Item kinds (`image`, `video`, `audio`, `document`, `code`, `link`, `archive`, `font`, `model`, `other`). |
| `exts` | `string[]` | No | `[]` | Lowercase extensions (`["png","webp"]`). |
| `color` | `string` | No | — | `#rrggbb` or `"grayscale"`. |
| `shapes` | `string[]` | No | `[]` | `landscape`, `portrait`, `square`, `wide`, `tall`, `4:3`, `16:9`. |
| `addedWithin` | `number` | No | — | Added within N days. |
| `addedFrom`, `addedTo` | `string` | No | — | `YYYY-MM-DD` date bounds on import date. |
| `modifiedFrom`, `modifiedTo` | `string` | No | — | `YYYY-MM-DD` date bounds on file modification date. |
| `createdFrom`, `createdTo` | `string` | No | — | `YYYY-MM-DD` date bounds on file creation date. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/setFilters \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"kinds":["image"],"ratings":[4,5],"shapes":["landscape"],"addedWithin":30}'
    ```

=== "JavaScript"

    ```javascript
    const view = await curio.view.setFilters({
      kinds: ['image'],
      ratings: [4, 5],
      shapes: ['landscape'],
      addedWithin: 30,
    })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"view_setFilters","arguments":{"kinds":["image"],"ratings":[5]}}}
    ```

**Example response**

```json
{ "show": "all", "id": null, "path": null, "name": "All", "search": "", "query": "kind = image AND (rating = 4 OR rating = 5) AND shape = landscape AND added WITHIN 30d", "count": 17, "view": "grid", "sample": { "text": "" } }
```

**Events and errors** Emits [`view.changed`](../events/windows.md#view-changed). Returns `400 bad_request` on invalid filter values, `403 not_allowed` without `ui`, `404 not_found` for unknown tag, folder or `window` IDs, `404 unknown_library`, `409 no_window` or `503 busy`.

---

## view.showWith {#view-showwith}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/view/showWith</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: view_showWith</span>
</div>

Switch the window to **All** filtered by a single tag or custom field value. This matches clicking a tag or field chip in Curio's Inspector, leaving the filter chip visible so the user can widen or clear it.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tag` | `string` | Either `tag` or (`field` + `value`) | — | Tag name (max 500 characters). |
| `field` | `string` | With `value` | — | Custom field key. |
| `value` | `string \| number` | With `field` | — | Field value (max 1,000 characters). |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`View`](../types/runtime.md#view).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/view/showWith \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"field":"lora","value":"detail_tweaker"}'
    ```

=== "JavaScript"

    ```javascript
    const view = await curio.view.showWith({ field: 'lora', value: 'detail_tweaker' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"view_showWith","arguments":{"tag":"portrait"}}}
    ```

**Example response**

```json
{ "show": "all", "id": null, "path": null, "name": "All", "search": "", "query": "lora = \"detail_tweaker\"", "count": 31, "view": "grid", "sample": { "text": "" } }
```

**Events and errors** Emits [`view.changed`](../events/windows.md#view-changed). Returns `400 bad_request` if neither valid combination is provided or limits are exceeded, `403 not_allowed` without `ui`, `404 not_found` if `field` or `window` does not exist, `404 unknown_library`, `409 no_window` or `503 busy`.
