# Completions

Hidden worker pages (`formats[].thumbnail_page` and `extractors[].page`) generate thumbnails (WebGL, Canvas, `pdf.js`, `foliate-js`) and extract searchable metadata using browser APIs with no external executables installed. Both require the `files.read` permission.

!!! note "Open window required"
    Hidden worker pages run inside the desktop WebView and do not execute in headless background holders. Items added while a library runs only in a background holder remain queued until a desktop window opens for that library.

## Thumbnail request

Curio mounts `thumbnail_page.page` in a hidden iframe and queues [`plugin.thumbnail`](../hooks/work.md#plugin-thumbnail) requests one at a time so startup delays never drop work. See [`plugin.thumbnail`](../hooks/work.md#plugin-thumbnail) for the full payload fields (`request`, `item`, `ext`, `size` and Range-capable `fileUrl`).

## curio.thumbnailDone(request, picture, info)

Answer every `plugin.thumbnail` event with `curio.thumbnailDone(request, picture, info)`, passing `null` on error so the queue does not wait for the 60-second timeout:

```javascript
const curio = Curio.connect()

curio.on('plugin.thumbnail', async ({ data }) => {
  try {
    const { blob, width, height } = await renderModelThumbnail(data.fileUrl, data.size)
    await curio.thumbnailDone(data.request, blob, { width, height })
  } catch (err) {
    await curio.plugin.log({ level: 'warn', message: `Thumb failed: ${err.message}` })
    await curio.thumbnailDone(data.request, null)
  }
})
```

## Picture formats

- **`picture`**: Accepts a `Blob` (for example from `canvas.toBlob`), a `data:image/...;base64,...` URI, a raw base64 string or `null`. Passing `null` reports that this page cannot render the item and lets Curio's fallback thumbnailers try next.
- **`info`** *(optional)*: `{ width?: number, height?: number }`. When supplied for visual media, Curio records those dimensions. Aspect ratio is derived from the rendered thumbnail so Masonry and Justified layouts keep the cover's proportions.

## Extract request

Curio mounts `extractors[].page` in a hidden iframe and queues [`plugin.extract`](../hooks/work.md#plugin-extract) requests one at a time. See [`plugin.extract`](../hooks/work.md#plugin-extract) for the full payload fields (`request`, `reader`, `item`, `ext`, `name`, `size`, `fileUrl` and companion `parts[]`).

For large binaries, read headers with `fetch(data.fileUrl, { headers: { Range: 'bytes=0-65535' } })` instead of loading the entire file into memory, or call `curio.item.exif`, `curio.item.pngText` or `curio.archive.list`.

## curio.extractDone(request, values, error)

```javascript
const curio = Curio.connect()

curio.on('plugin.extract', async ({ data }) => {
  try {
    const metaPart = data.parts.find((p) => p.role === 'metadata' && p.name.endsWith('.json'))
    const meta = metaPart ? await fetch(metaPart.fileUrl).then((r) => r.json()) : {}
    await curio.extractDone(data.request, {
      base_model: meta.baseModel || null,
      trigger_words: meta.trainedWords || [],
      __source_url: meta.modelUrl || null,
    })
  } catch (err) {
    await curio.extractDone(data.request, null, err.message)
  }
})
```

- **Field filtering**: Only field keys declared in `extractors[].fields` (plus `__title`, `__annotation` and `__source_url` for empty item metadata) are stored. Listed fields omitted from `values` are cleared on rescan, while manual user overrides are preserved.
- **Reply size**: Extraction replies must stay under **4 MB**.

## Timeouts

- **60-second worker limit**: Any thumbnail or extractor request unanswered after 60 seconds times out (and records a failure in **Settings › Fields** for extractors). Idle worker frames close 30 seconds after their last job to free WebGL contexts and decoders.
- **Failure circuit breaker**: Five consecutive failures disable the plugin until re-enabled. See [Loading › Failures](../start/loading.md#failures) for all runner timeouts and recovery steps.
