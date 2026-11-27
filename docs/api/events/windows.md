# Windows

Window-level UI events requiring the `ui` permission. Every event carries a `window` label identifying which open Curio window triggered it.

Subscribe over HTTP SSE by sending event names or wildcards (`{"events":["selection.*","view.*","viewer.*"]}`) to `POST /api/v1/events`, or use `curio.on(...)` below. See [Subscribe to the event stream](../connect/events.md) for stream filtering, heartbeats, replay windows and reconnection.

## selection.changed {#selection-changed}

<div class="curio-meta">
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">First 1,000 selected IDs</span>
  <span class="curio-badge curio-badge--muted">Wildcard: selection.*</span>
</div>

Emitted when the user or [`selection.set`](../operations/selection.md#selection-set) changes the selected items in a window.

**Payload** (`{ window: string; count: number; ids: string[] }`)

| Field | Type | Description |
|---|---|---|
| `window` | `string` | Window label (e.g. `"main"`). |
| `count` | `number` | Total number of selected items (may exceed `ids.length` when more than 1,000 items are selected). |
| `ids` | `string[]` | Up to the first 1,000 selected item UUIDs. |

```javascript
curio.on('selection.changed', (ev) => {
  console.log(ev.data.window, 'selected:', ev.data.count, ev.data.ids)
})
```

**Observed event**

```json
{
  "id": "1763941500000-a1b2c3d4-40",
  "name": "selection.changed",
  "library": "c4b8f1d2-...",
  "at": 1763941500000,
  "data": { "window": "main", "count": 2, "ids": ["0194b7a2-...", "0194b7a3-..."] }
}
```

---

## view.changed {#view-changed}

<div class="curio-meta">
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Scope: Window / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: view.*</span>
</div>

Emitted when a window opens another folder, tag, location, quick view or plugin tab. Also emitted when search, Filter bar chips or font sample text change.

**Payload** (`View & { window: string }`): Carries `window` (the window label) plus every property of [`View`](../types/runtime.md#view), matching [`view.get`](../operations/view.md#view-get).

```javascript
curio.on('view.changed', (ev) => {
  console.log(`${ev.data.window}: ${ev.data.name} (${ev.data.count} items)`)
})
```

**Observed event**

```json
{
  "id": "1763941505000-a1b2c3d4-41",
  "name": "view.changed",
  "library": "c4b8f1d2-...",
  "at": 1763941505000,
  "data": { "window": "main", "show": "folder", "id": 3, "path": null, "name": "References", "search": "", "query": "", "count": 120, "view": "grid", "sample": { "text": "" } }
}
```

---

## viewer.opened {#viewer-opened}

<div class="curio-meta">
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Scope: Window / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: viewer.*</span>
</div>

Emitted when a window's viewer opens an item, steps between items or switches to another member file in a grouped item.

**Payload** (`{ window: string; id: string; file: string }`)

| Field | Type | Description |
|---|---|---|
| `window` | `string` | Window label. |
| `id` | `string` | Primary item UUID. |
| `file` | `string` | Specific file UUID shown (equals `id` unless viewing a grouped member). |

```javascript
curio.on('viewer.opened', (ev) => console.log('Viewing:', ev.data.id, 'file:', ev.data.file))
```

**Observed event**

```json
{
  "id": "1763941510000-a1b2c3d4-42",
  "name": "viewer.opened",
  "library": "c4b8f1d2-...",
  "at": 1763941510000,
  "data": { "window": "main", "id": "0194b7a2-...", "file": "0194b7a2-..." }
}
```

---

## viewer.closed {#viewer-closed}

<div class="curio-meta">
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Scope: Window / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: viewer.*</span>
</div>

Emitted when a window closes its viewer back to the grid.

**Payload** (`{ window: string; id: string }`)

| Field | Type | Description |
|---|---|---|
| `window` | `string` | Window label. |
| `id` | `string` | UUID of the item that was open last. |

```javascript
curio.on('viewer.closed', (ev) => console.log('Closed viewer for:', ev.data.id))
```

**Observed event**

```json
{
  "id": "1763941520000-a1b2c3d4-43",
  "name": "viewer.closed",
  "library": "c4b8f1d2-...",
  "at": 1763941520000,
  "data": { "window": "main", "id": "0194b7a2-..." }
}
```
