# Files

Plugin pages can read and write item bytes through four mechanisms without exhausting memory or base64-encoding large files.

## Blobs

With `files.read`, [`item.thumbnail`](../../api/operations/item-files.md#item-thumbnail), [`item.file`](../../api/operations/item-files.md#item-file) and [`archive.entry`](../../api/operations/archive.md#archive-entry) resolve to a standard browser `Blob` (up to 512 MB):

```javascript
const blob = await curio.item.file({ id: item.id })
const text = await blob.text()
```

Use `Blob` reads for small files (text, CSV, JSON, thumbnails). For large videos, PDFs, ebooks, 3D models or high-resolution images, use URL streaming below.

## Range URLs

Call [`item.fileUrl`](../../api/operations/item-files.md#item-fileurl) to get a same-origin URL served by Curio's host with full HTTP `GET` and `Range` support:

```javascript
const { url } = await curio.item.fileUrl({ id: item.id })
// Works with <video>, <audio>, pdf.js, Three.js or partial Range fetches:
const head = await fetch(url, { headers: { Range: 'bytes=0-4095' } })
```

The URL stays valid for the item even if its file moves between managed folders. Files served through `fileUrl` never execute as HTML documents.

## Companions

For extensions declared in your plugin's `formats[]`, Curio resolves sibling and grouped files relative to the `item.fileUrl` path (up to 3 subdirectories deep, excluding hidden files and folders):

- **Allowed files**: Images, `.mtl`, `.bin`, GPU texture formats (`tga`, `dds`, `ktx`, `ktx2`, `hdr`, `exr`), any extension declared by your plugin and any file grouped into the item by companion rules.
- **Relative loader support**: Standard loaders such as Three.js `OBJLoader` + `MTLLoader` or `GLTFLoader` can fetch external `.mtl`, `.bin` and texture maps with no custom URL rewriting.

## Previews

Avoid decoding a 12,000 px JPEG, camera RAW or PSD at full resolution to display it at screen size. Call [`item.previewUrl`](../../api/operations/item-files.md#item-previewurl) with `size` set to the needed pixel dimension (`Math.ceil(boxPx * devicePixelRatio * zoom)`, rounded up in steps of `256` from `256` to `8192`):

```javascript
const { url } = await curio.item.previewUrl({ id: item.id, size: 1536 })
img.src = url
```

- Works across every visual format Curio decodes, including camera RAW and PSD.
- Cached in Curio's 192 MiB in-memory preview LRU (nothing is written to disk). The URL stays stable while the item's file is unchanged so browser `<img>` decoding reuses it.

## Archives

For ZIP, CBZ, EPUB, 7z and CB7 items:

1. Call [`archive.list`](../../api/operations/archive.md#archive-list) to inspect every entry in natural reading order, including picture `width`, `height` and `animated` flags sniffed from file headers so a reader can compute page layouts before loading images.
2. Call [`archive.entryUrl`](../../api/operations/archive.md#archive-entryurl) with an optional `size` to stream scaled page images directly into `<img>` tags. Animated images and images already smaller than `size` are served untouched, and 7z archives unpack once on demand.

## Writing

With `files.write`, write binary output from a `<canvas>`, WebAssembly encoder or `Blob` via [`item.writeUrl`](../../api/operations/item-files.md#item-writeurl) and HTTP `PUT` without base64 overhead:

```javascript
const blob = await canvas.convertToBlob({ type: 'image/webp', quality: 0.85 })
const { url } = await curio.item.writeUrl({ id: item.id, mode: 'beside', name: 'converted.webp' })
const reply = await fetch(url, { method: 'PUT', body: blob }).then((r) => r.json())
if (reply.error) throw new Error(reply.error.message)
console.log('Created item IDs:', reply.ok.ids)
```

- **`mode: "replace"`** *(default)*: Replaces the item's file in place (same type; the old file moves to the Recycle Bin).
- **`mode: "beside"`**: Creates a sibling item inheriting folders, tags, rating, notes, source URL and custom fields.
- **Single-use URL**: Each write URL accepts one `PUT` and then expires. For small base64 payloads, [`item.replaceFile`](../../api/operations/item-files.md#item-replacefile) and [`item.add.beside`](../../api/operations/item-add.md#item-add-beside) are also available.
