# plugin

Plugin-exclusive operations for storing private library-scoped JSON state, per-item private state, computer-scoped plugin settings, structured logging and closing plugin windows or popovers. Every operation requires only the plugin's implicit `library.read` permission and is isolated to the calling plugin's own ID. None of these operations are exposed to external web tokens or MCP.

## plugin.state.get {#plugin-state-get}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Read a private JSON value stored by this plugin for the current library under `key` (`null` if unset).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Storage key within this plugin's namespace. |

**Returns** `{ value: unknown }`.

**Example request**

=== "Plugin page"

    ```javascript
    const { value } = await curio.plugin.state.get({ key: 'lastPreset' })
    ```

**Example response**

```json
{ "value": { "format": "webp", "quality": 88 } }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside a plugin caller.

---

## plugin.state.set {#plugin-state-set}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Store any JSON value (up to **1 MiB**) for this plugin in the current library. Pass `value: null` to delete the key.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `key` | `string` | Yes | — | Storage key (1–200 characters). |
| `value` | `unknown` | Yes | — | Any JSON-serializable value (≤ 1 MiB), or `null` to delete. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.plugin.state.set({ key: 'lastPreset', value: { format: 'webp', quality: 88 } })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits no library events. Returns `400 bad_request` if `key` is empty or exceeds 200 characters, or serialized JSON exceeds 1 MiB; `403 not_allowed` outside a plugin caller.

---

## plugin.itemState.get {#plugin-itemstate-get}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Read private data stored by this plugin for one item (such as a Reader page position or bookmarks); returns `{ value: null }` when none exists.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |

**Returns** `{ value: unknown }`.

**Example request**

=== "Plugin page"

    ```javascript
    const { value } = await curio.plugin.itemState.get({ id: item.id })
    ```

**Example response**

```json
{ "value": { "page": 14, "bookmarks": [{ "page": 3, "name": "Map" }] } }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside a plugin caller, or `404 not_found` if `id` does not exist.

---

## plugin.itemState.getMany {#plugin-itemstate-getmany}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Batch-read private plugin item state for multiple items at once. Items with no saved state are omitted from the returned object.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item IDs to read. |

**Returns** `Record<string, unknown>` (keyed by item UUID).

**Example request**

=== "Plugin page"

    ```javascript
    const states = await curio.plugin.itemState.getMany({ ids: [book1, book2] })
    ```

**Example response**

```json
{ "0194b7a2-...": { "page": 14 } }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside a plugin caller, or `404 not_found` if any ID in `ids` does not exist.

---

## plugin.itemState.set {#plugin-itemstate-set}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Store private JSON data (up to **64 KiB**) attached to an item, automatically deleted if the item is purged. Pass `value: null` to remove it. Emits **no** `items.changed` event, so saving scroll or reading progress frequently never causes UI redraws.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string \| number` | Yes | — | Item UUID or row ID. |
| `value` | `unknown` | Yes | — | JSON value (≤ 64 KiB), or `null` to delete. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.plugin.itemState.set({ id: item.id, value: { page: 15 } })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits no events. Returns `400 bad_request` if payload exceeds 64 KiB, `403 not_allowed` outside a plugin caller or `404 not_found` if `id` does not exist.

---

## plugin.settings.get {#plugin-settings-get}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App (per computer)</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Read the plugin's computer-wide settings (shared across all libraries on this machine).

**Parameters** None (`{}`).

**Returns** `Record<string, unknown>`.

**Example request**

=== "Plugin page"

    ```javascript
    const settings = await curio.plugin.settings.get()
    ```

**Example response**

```json
{ "theme": "sepia", "fontSize": 18, "trackProgress": true }
```

**Events and errors** Read-only. Returns `403 not_allowed` outside a plugin caller.

---

## plugin.settings.set {#plugin-settings-set}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App (per computer)</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Merge key-value updates into the plugin's computer-wide settings (up to **1 MiB** total; `null` deletes a key). All other pages of this plugin across open Curio windows receive [`plugin.settingsChanged`](../../plugins/hooks/lifecycle.md#plugin-settingschanged).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `values` | `Record<string, unknown>` | Yes | — | Keys to set or clear (`null`). |

**Returns** `Record<string, unknown>` — the full merged settings object.

**Example request**

=== "Plugin page"

    ```javascript
    const updated = await curio.plugin.settings.set({ values: { fontSize: 20, oldKey: null } })
    ```

**Example response**

```json
{ "theme": "sepia", "fontSize": 20, "trackProgress": true }
```

**Events and errors** Delivers [`plugin.settingsChanged`](../../plugins/hooks/lifecycle.md#plugin-settingschanged) to sibling pages of the plugin. Returns `400 bad_request` if total settings exceed 1 MiB, `403 not_allowed` outside a plugin caller or `422 failed` if the settings file cannot be written.

---

## plugin.log {#plugin-log}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Append a line tagged with this plugin's ID to Curio's diagnostic log (`curio.log`).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `message` | `string` | Yes | — | Log line text. |
| `level` | `string` | No | `"info"` | `info`, `warn` or `error`. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.plugin.log({ level: 'warn', message: 'Fallback parser used for legacy header' })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Writes to `curio.log`. Returns `403 not_allowed` outside a plugin caller.

---

## plugin.close {#plugin-close}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App (host window)</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin window or popover pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Close the standalone plugin window hosting this page, or close a popover opened via [`ui.popover`](ui.md#ui-popover) and resolve the caller's promise with `result`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `result` | `unknown` | No | `null` | Value returned to the `ui.popover` caller when closing a popover. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.plugin.close({ result: { picked: 'option-b' } })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Returns `403 not_allowed` if called from a widget, tab or viewer page.
