# State

Curio provides four storage scopes for plugins, all isolated to your plugin's ID and covered by implicit `library.read` with no extra permissions.

| Storage API | Scope | Size limit | Visible/searchable by user | Triggers `items.changed` | Survives library move |
|---|---|---|---|---|---|
| [`plugin.state`](../../api/operations/plugin.md#plugin-state-set) | Per library, per key | 1 MB / key | No (private JSON) | No | Yes (in `library.db`) |
| [`plugin.itemState`](../../api/operations/plugin.md#plugin-itemstate-set) | Per library, per item | 64 KB / item | No (private JSON) | No | Yes (deleted with item) |
| [`plugin.settings`](../../api/operations/plugin.md#plugin-settings-set) | Per computer (all libraries) | Config JSON | Via your `settings` page | Fires `plugin.settingsChanged` | No (machine config) |
| [`fields[]`](../contributions/fields.md) | Per library, per item | Typed EAV | **Yes** (Inspector, Search, Filter, Sort, Export) | **Yes** | Yes (in `library.db`) |

## plugin.state

Use `plugin.state.get({ key })` and `plugin.state.set({ key, value })` for library-specific configuration, saved presets, caches or last viewport state (up to **1 MB** per key):

```javascript
const { value } = await curio.plugin.state.get({ key: 'presets' })
const presets = value || []
presets.push({ name: 'WebP 85', quality: 0.85 })
await curio.plugin.state.set({ key: 'presets', value: presets })
```

## plugin.itemState

Use `plugin.itemState.get({ id })`, `plugin.itemState.getMany({ ids })` and `plugin.itemState.set({ id, value })` for high-frequency or opaque per-item state (up to **64 KB** per item), such as reading positions, scroll offsets, 3D camera angles or crop rectangles:

```javascript
// Batch-read progress across a whole series widget in one call
const progressMap = await curio.plugin.itemState.getMany({ ids: seriesIds })

// Save reading position every few seconds without refreshing Curio's main grid
await curio.plugin.itemState.set({ id: bookId, value: { cfi: currentCfi, zoom: 1.1 } })
```

When an item is permanently deleted, Curio deletes its `plugin.itemState` rows with it.

## plugin.settings

Use `plugin.settings.get()` and `plugin.settings.set({ values })` for user preferences shared across every library on the same computer (such as Reader typography defaults, 3D viewer shading defaults or converter presets):

```javascript
const prefs = await curio.plugin.settings.get()
await curio.plugin.settings.set({ values: { fontSize: 18, theme: 'sepia' } })
```

Calling `plugin.settings.set` notifies every other open page of your plugin across all windows (including **Settings › Plugins**) via [`plugin.settingsChanged`](../hooks/lifecycle.md#plugin-settingschanged).

## Fields vs state

- **Store in `fields[]`**: Values the user may search, filter, sort, group by, inspect in columns or export (such as `read_state: "finished"`, `read_progress: 100`, `model_triangles`, `author` or `prompt`).
- **Store in `plugin.itemState`**: Internal UI state that should not clutter the Inspector or trigger search index updates (such as scroll offsets, EPUB CFI strings, 3D camera quaternions or collapsed panel flags).

## Limits & events

- **Deleting keys**: Pass `null` as `value` in `plugin.state.set` or `plugin.itemState.set`, or for any key in `plugin.settings.set({ values: { oldKey: null } })`, to delete that key.
- **Size errors**: Exceeding **1 MB** (`plugin.state`) or **64 KB** (`plugin.itemState`) rejects with `400 bad_request`.

## Logs & storage

- **Logs**: Write diagnostic lines to `curio.log` with [`plugin.log({ level, message })`](../../api/operations/plugin.md#plugin-log).
- **WebView storage**: `localStorage` and IndexedDB work inside your plugin's isolated origin, but treat them as disposable caches because WebView storage can be cleared independently of library backups.
