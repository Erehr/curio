# Scans

Events published when indexed locations change or go offline, and while a directory scan runs and completes.

Subscribe over HTTP SSE by sending `{"events":["locations.changed","scan.*"]}` to `POST /api/v1/events`, or use `curio.on(...)` below. See [Subscribe to the event stream](../connect/events.md) for stream filtering, heartbeats, replay windows and reconnection.

## locations.changed {#locations-changed}

<div class="curio-meta">
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Invalidation signal</span>
  <span class="curio-badge curio-badge--muted">Wildcard: locations.*</span>
</div>

Emitted when a location is added, removed, relocated, updated (`setAccess` or `setLook`), finishes a scan or switches between online and offline. Payload is `{}`. Call [`location.list`](../operations/location.md#location-list) to refresh.

```javascript
curio.on('locations.changed', async () => {
  const locs = await curio.location.list()
  console.log(locs.map((l) => `${l.name}: ${l.online ? 'online' : 'offline'}`))
})
```

**Observed event**

```json
{ "id": "1763941400000-a1b2c3d4-30", "name": "locations.changed", "library": "c4b8f1d2-...", "at": 1763941400000, "data": {} }
```

---

## scan.progress {#scan-progress}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Throttled ~1/sec</span>
  <span class="curio-badge curio-badge--muted">Wildcard: scan.*</span>
</div>

Emitted once when a location scan starts (`files: 0`) and roughly once per second while walking the directory tree.

**Payload** (`{ location: number; files: number }`)

| Field | Type | Description |
|---|---|---|
| `location` | `number` | Numeric location ID being scanned. |
| `files` | `number` | Number of files visited so far. |

```javascript
curio.on('scan.progress', (ev) => {
  console.log(`Location ${ev.data.location}: ${ev.data.files} files seen`)
})
```

**Observed event**

```json
{
  "id": "1763941401100-a1b2c3d4-31",
  "name": "scan.progress",
  "library": "c4b8f1d2-...",
  "at": 1763941401100,
  "data": { "location": 2, "files": 1240 }
}
```

---

## scan.done {#scan-done}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: scan.*</span>
</div>

Emitted when a location scan completes with a summary of discovered changes.

**Payload** (`{ location: number; files: number; added: number; changed: number; moved: number; missing: number; offline: number }`)

| Field | Type | Description |
|---|---|---|
| `location` | `number` | Location ID. |
| `files` | `number` | Total files seen during the scan. |
| `added` | `number` | Newly indexed items. |
| `changed` | `number` | Existing items whose file size or modification time changed. |
| `moved` | `number` | Existing items matched at a new path within the location. |
| `missing` | `number` | Items marked Missing because their file disappeared. |
| `offline` | `number` | Items marked Storage offline. |

```javascript
curio.on('scan.done', (ev) => console.log('Scan finished:', ev.data))
```

**Observed event**

```json
{
  "id": "1763941405900-a1b2c3d4-32",
  "name": "scan.done",
  "library": "c4b8f1d2-...",
  "at": 1763941405900,
  "data": { "location": 2, "files": 1520, "added": 14, "changed": 2, "moved": 1, "missing": 0, "offline": 0 }
}
```
