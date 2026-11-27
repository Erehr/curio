# Items

Events published when items are added, modified or removed, or when folders, smart folders, tags, tag groups or custom field definitions change. All eight events require `metadata.read` and are scoped to a library. Large batches of `items.*` events split into chunks of at most **1,000 IDs** per event.

Subscribe over HTTP SSE by sending event names or wildcards (`{"events":["items.*","folders.changed"]}`) to `POST /api/v1/events`, or use `curio.on(...)` below. See [Subscribe to the event stream](../connect/events.md) for stream filtering, heartbeats, replay windows and reconnection.

## items.added {#items-added}

<div class="curio-meta">
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Max 1,000 IDs / event</span>
  <span class="curio-badge curio-badge--muted">Wildcard: items.*</span>
</div>

Emitted when new items enter a library via the desktop UI, API import, Auto-Import, location scan, duplication or Trash restore.

**Payload** (`{ ids: string[]; how: 'window' | 'api' | 'autoImport' | 'scan' | 'duplicate' | 'restore'; requestId?: string }`)

| Field | Type | Description |
|---|---|---|
| `ids` | `string[]` | Up to 1,000 newly added item UUIDs. |
| `how` | `string` | Origin of the addition: `window`, `api`, `autoImport`, `scan`, `duplicate` or `restore`. |
| `requestId` | `string \| undefined` | Present when triggered by an API `item.add.*` call carrying `requestId`. |

```javascript
curio.on('items.added', (ev) => {
  console.log(ev.data.how, ev.data.ids.length, ev.data.requestId)
})
```

**Observed event**

```json
{
  "id": "1763941300100-a1b2c3d4-20",
  "name": "items.added",
  "library": "c4b8f1d2-...",
  "at": 1763941300100,
  "data": { "ids": ["0194b7c1-..."], "how": "api", "requestId": "req-42" }
}
```

---

## items.changed {#items-changed}

<div class="curio-meta">
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Max 1,000 IDs / event · Thumbnails coalesced 500 ms</span>
  <span class="curio-badge curio-badge--muted">Wildcard: items.*</span>
</div>

Emitted when existing items' metadata, files on disk, thumbnails or color palettes change. Thumbnail completions coalesce over a **500 ms** window into batched events. Calling [`plugin.itemState.set`](../operations/plugin.md#plugin-itemstate-set) emits no `items.changed`.

**Payload** (`{ ids: string[]; what: 'metadata' | 'file' | 'thumbnail' | 'palette' }`)

| Field | Type | Description |
|---|---|---|
| `ids` | `string[]` | Up to 1,000 changed item UUIDs. |
| `what` | `'metadata' \| 'file' \| 'thumbnail' \| 'palette'` | Aspect that changed. |

```javascript
curio.on('items.changed', (ev) => {
  if (ev.data.what === 'metadata') console.log('Metadata edited:', ev.data.ids)
})
```

**Observed event**

```json
{
  "id": "1763941302400-a1b2c3d4-21",
  "name": "items.changed",
  "library": "c4b8f1d2-...",
  "at": 1763941302400,
  "data": { "ids": ["0194b7a2-..."], "what": "thumbnail" }
}
```

---

## items.removed {#items-removed}

<div class="curio-meta">
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Max 1,000 IDs / event</span>
  <span class="curio-badge curio-badge--muted">Wildcard: items.*</span>
</div>

Emitted when items move to Trash (`purged: false`) or are permanently deleted (`purged: true`).

**Payload** (`{ ids: string[]; purged: boolean }`)

| Field | Type | Description |
|---|---|---|
| `ids` | `string[]` | Up to 1,000 affected item UUIDs. |
| `purged` | `boolean` | `false` when moved to Trash; `true` when permanently removed. |

```javascript
curio.on('items.removed', (ev) => console.log('Removed:', ev.data.ids, 'purged:', ev.data.purged))
```

**Observed event**

```json
{
  "id": "1763941309000-a1b2c3d4-22",
  "name": "items.removed",
  "library": "c4b8f1d2-...",
  "at": 1763941309000,
  "data": { "ids": ["0194b7a2-..."], "purged": false }
}
```

---

## Invalidation events

<div class="curio-meta">
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Library</span>
  <span class="curio-badge curio-badge--muted">Payload: {}</span>
</div>

These five library-scoped events signal that a structural list changed. Each carries an empty `{}` payload, so call the matching list operation to refresh your copy.

```javascript
curio.on('folders.changed', async () => {
  const folders = await curio.folder.list()
  console.log('Folder count:', folders.length)
})
```

```json
{ "id": "1763941315000-a1b2c3d4-23", "name": "folders.changed", "library": "c4b8f1d2-...", "at": 1763941315000, "data": {} }
```

### folders.changed {#folders-changed}

Wildcard `folders.*`. Emitted when folders are created, renamed, moved, deleted, restyled or change item counts. Refresh with [`folder.list`](../operations/folder.md#folder-list).

### smartFolders.changed {#smartfolders-changed}

Wildcard `smartFolders.*`. Emitted when smart folders are created, updated or deleted. Refresh with [`smartFolder.list`](../operations/smart-folder.md#smartfolder-list).

### tags.changed {#tags-changed}

Wildcard `tags.*`. Emitted when tags are created, renamed, merged, deleted, colored, starred, assigned or removed. Refresh with [`tag.list`](../operations/tag.md#tag-list).

### tagGroups.changed {#taggroups-changed}

Wildcard `tagGroups.*`. Emitted when tag groups are created, edited, deleted or change member tags. Refresh with [`tagGroup.list`](../operations/tag-group.md#taggroup-list).

### fields.changed {#fields-changed}

Wildcard `fields.*`. Emitted when custom or plugin field definitions are created, modified, switched on or off, cleaned up or deleted. Refresh with [`field.list`](../operations/field.md#field-list).
