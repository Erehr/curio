# Interaction

Page-local hooks notify viewer pages, widgets and standalone windows of active items, selection changes, drag-and-drop gestures and native viewer toolbar interactions.

## plugin.item {#plugin-item}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Sticky</span>
  <span class="curio-badge curio-badge--muted">Roles: viewer</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event to a format plugin's `viewer` page on first subscribe and whenever the user steps to another item handled by that viewer. Curio keeps the viewer iframe mounted across adjacent items.

**Payload**

| Field | Type | Description |
|---|---|---|
| `id` | `string` | Item UUIDv7. |
| `kind` | `string` | Item kind (`"document"`, `"model"`, `"archive"`, etc.). |
| `ext` | `string` | Lowercase extension without dot. |
| `name` | `string` | Display title. |

```javascript
curio.on('plugin.item', async ({ data }) => {
  const { url } = await curio.item.fileUrl({ id: data.id })
  await openDocument(data.id, url)
})
```

---

## plugin.selection {#plugin-selection}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Sticky</span>
  <span class="curio-badge curio-badge--muted">Roles: widget</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event to a page widget (`widgets[].page`) on first subscribe and whenever the window's selection changes.

**Payload**

| Field | Type | Description |
|---|---|---|
| `count` | `number` | Total number of selected items. |
| `ids` | `string[]` | Up to the first 1,000 selected item UUIDs. |
| `item` | `{ id: string; kind: string; ext: string; name: string } \| null` | Populated when `count === 1`; `null` otherwise. |

```javascript
curio.on('plugin.selection', async ({ data }) => {
  if (!data.count) return (document.body.innerHTML = '')
  const items = await curio.item.getMany({ ids: data.ids })
  renderWidget(items)
})
```

---

## plugin.drop {#plugin-drop}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: window, tab, widget, viewer</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event when the user drops items from Curio's grid onto a plugin frame, or drops OS files or items from another Curio window onto a standalone plugin window. Curio imports unindexed OS files into the library before delivering their item IDs.

**Payload**

| Field | Type | Description |
|---|---|---|
| `items` | `string[]` | Dropped item UUIDs (up to 1,000). |
| `x`, `y` | `number \| undefined` | Drop coordinates in CSS pixels relative to the plugin iframe's top-left corner. |

```javascript
curio.on('plugin.drop', ({ data }) => {
  const targetSlot = data.x < window.innerWidth / 2 ? 'left' : 'right'
  assignToSlot(targetSlot, data.items)
})
```

---

## plugin.dragOver {#plugin-dragover}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: window</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event continuously while files are dragged over a standalone plugin window (for drop-zone highlighting) and once with `{}` when the drag leaves or completes.

**Payload**: `{ x?: number, y?: number }` (in CSS pixels relative to the frame; empty object `{}` on leave or drop).

```javascript
curio.on('plugin.dragOver', ({ data }) => {
  dropZone.classList.toggle('active', data.x != null)
})
```

---

## plugin.tool {#plugin-tool}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: viewer</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event to a viewer page when the user interacts with a toolbar control published via [`viewer.setTools`](../../api/operations/viewer.md#viewer-settools) or presses an active shortcut from `viewer_keys[]`.

**Payload**

| Interaction | `data` shape |
|---|---|
| Toolbar button clicked | `{ id: string }` |
| Toolbar button right-clicked or long-pressed | `{ id: string, menu: true }` (a subsequent `ui.menu` anchors under that button) |
| Viewer key pressed (`viewer_keys[]`) | `{ id: string, key: true }` |
| Toolbar slider or button panel control changed | `{ id: string, value: string \| number \| boolean }` |

```javascript
curio.on('plugin.tool', async ({ data }) => {
  if (data.menu && data.id === 'bookmark') {
    const { id } = await curio.ui.menu({ items: [{ id: 'clear', label: 'Clear bookmarks' }] })
    if (id === 'clear') clearBookmarks()
    return
  }
  if (data.value !== undefined) setSetting(data.id, data.value)
  else runToolAction(data.id)
})
```
