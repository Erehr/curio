# Work

Queued hooks arrive in hidden background worker frames (`formats[].thumbnail_page` and `extractors[].page`). Curio queues both hooks until your script calls `curio.on(...)` so startup requests are never dropped. Answer every request with its completion helper (`curio.thumbnailDone` or `curio.extractDone`) within **60 seconds**.

See [Complete a thumbnail or extraction request](../pages/completions.md) for full rendering patterns, accepted picture formats and error-handling guidance.

## plugin.thumbnail {#plugin-thumbnail}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Queued</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Roles: thumbnail</span>
  <span class="curio-badge curio-badge--muted">Reply: curio.thumbnailDone()</span>
</div>

Curio sends this event when an item matching your plugin's `formats[].thumbnail_page` needs a rendered thumbnail.

**Payload**

| Field | Type | Description |
|---|---|---|
| `request` | `number` | Opaque request ID to pass back to `curio.thumbnailDone`. |
| `item` | `string` | Item UUIDv7. |
| `ext` | `string` | Lowercase extension without dot. |
| `size` | `number` | Target maximum edge in pixels (from `thumbnail_page.size`). |
| `fileUrl` | `string` | Range-capable URL for reading the file and relative companions. |

**Reply contract**

```javascript
const curio = Curio.connect()
curio.on('plugin.thumbnail', async ({ data }) => {
  try {
    const blob = await renderCover(data.fileUrl, data.size)
    await curio.thumbnailDone(data.request, blob, { width: 512, height: 512 })
  } catch {
    await curio.thumbnailDone(data.request, null)
  }
})
```

See [Complete a thumbnail or extraction request](../pages/completions.md) for supported picture types and fallback behavior.

---

## plugin.extract {#plugin-extract}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Queued</span>
  <span class="curio-badge">Permission: files.read</span>
  <span class="curio-badge curio-badge--muted">Roles: extractor</span>
  <span class="curio-badge curio-badge--muted">Reply: curio.extractDone()</span>
</div>

Curio sends this event when an item matching your plugin's `extractors[].page` needs metadata extraction.

**Payload**

| Field | Type | Description |
|---|---|---|
| `request` | `number` | Opaque request ID to pass back to `curio.extractDone`. |
| `reader` | `string` | Full extractor ID (`"<plugin_id>/<extractor_id>"`). |
| `item` | `string` | Item UUIDv7. |
| `ext` | `string` | Lowercase extension without dot. |
| `name` | `string` | Filename on disk. |
| `size` | `number` | File size in bytes. |
| `fileUrl` | `string` | Range-capable URL for reading the primary file. |
| `parts` | `{ role: string; name: string; item: string; size: number; fileUrl: string }[]` | Grouped companion files attached to this item. |

**Reply contract**

```javascript
const curio = Curio.connect()
curio.on('plugin.extract', async ({ data }) => {
  try {
    const text = await fetch(data.fileUrl).then((r) => r.text())
    const lines = text.split(/\r?\n/)
    await curio.extractDone(data.request, {
      first_line: lines[0] || '',
      lines: lines.length,
    })
  } catch (err) {
    await curio.extractDone(data.request, null, err.message)
  }
})
```

Reply payloads are capped at **4 MB**. Unanswered requests time out after 60 seconds, and Curio logs them in **Settings › Fields**. See [Complete a thumbnail or extraction request](../pages/completions.md) for Range requests, clearing rules and error handling.
