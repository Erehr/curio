# Files & uploads

Curio supports binary reads, 2 GiB spool uploads, plugin range streaming, single-use write URLs and queued imports across HTTP, `curio.js`, MCP and plugin pages.

## Binary reads

Three operations return raw file bytes instead of JSON:

| Operation | HTTP response | `curio.js` return | MCP response |
|---|---|---|---|
| [`item.thumbnail`](../operations/item-files.md#item-thumbnail) | Raw image bytes (`image/jpeg`, `image/png` or `image/webp`) | `Blob` | `image` content block (base64 + `mimeType`, up to **4 MiB**; larger images return an error) |
| [`item.file`](../operations/item-files.md#item-file) | Raw item bytes with detected MIME `Content-Type` | `Blob` | Not exposed as an MCP tool |
| [`archive.entry`](../operations/archive.md#archive-entry) | Raw archive member bytes | `Blob` | Not exposed as an MCP tool |

```javascript
const blob = await curio.item.thumbnail({ id: item.id })
img.src = URL.createObjectURL(blob)
```

## Uploads

Import raw bytes from an external script or browser extension without writing to disk first:

- **Small files (up to ~44 MiB)**: Pass base64 (or a `data:...;base64,...` URI) in `data` to [`item.add.fromBytes`](../operations/item-add.md#item-add-frombytes), fitting within the 64 MiB JSON body cap.
- **Large files (up to 2 GiB)**: Stream raw bytes to `POST /api/v1/upload?name=<filename>` to get an upload ticket ID, then pass `upload` to [`item.add.fromBytes`](../operations/item-add.md#item-add-frombytes).

=== "HTTP"

    ```bash
    # 1. Stream raw bytes to the spool
    curl -s -X POST "http://127.0.0.1:41733/api/v1/upload?name=clip.mp4" \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      --data-binary @clip.mp4
    # -> {"upload":"9f86d081884c7d659a2feaa0c55ad015","size":157286400}

    # 2. Import the spooled upload into a library
    curl -s -X POST "http://127.0.0.1:41733/api/v1/item/add/fromBytes" \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"upload":"9f86d081884c7d659a2feaa0c55ad015","name":"clip.mp4","tags":["renders"]}'
    ```

=== "JavaScript"

    ```javascript
    const Curio = require('./curio.js') // Downloaded from /api/v1/curio.js
    const fs = require('node:fs')

    const token = process.env.CURIO_TOKEN
    const curio = Curio.connect({ token })

    // 1. Stream raw bytes to the spool
    const up = await fetch('http://127.0.0.1:41733/api/v1/upload?name=clip.mp4', {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/octet-stream' },
      body: fs.readFileSync('clip.mp4'),
    })
    const { upload } = await up.json()

    // 2. Import the spooled upload into a library
    await curio.item.add.fromBytes({ upload, name: 'clip.mp4', tags: ['renders'] })
    ```

## Upload limits

Curio stores spooled uploads in its temporary `spool/` folder and enforces these limits:

| Rule | Limit or behavior |
|---|---|
| Permission required | Requires a web API token holding `files.write`. |
| Maximum file size | **2 GiB** per upload. Empty uploads return `400 bad_request`. |
| Free disk space margin | Curio requires at least **512 MiB** of free space remaining on its data drive after the upload (`413 too_large` otherwise). |
| Token binding & single use | Only the exact bearer token that uploaded the file can claim its 32-hex-character upload ID. Claiming moves the file out of the spool so a second claim fails. |
| Expiry | Curio removes unclaimed uploads after **10 minutes**. |

## Range URLs

Plugin pages should avoid loading large videos, PDFs or archives into memory as Blobs. Use Curio's page-only URL operations instead:

- **[`item.fileUrl`](../operations/item-files.md#item-fileurl)**: Returns a scoped URL supporting HTTP `GET` and `Range` requests for `<video>`, `<audio>`, `pdf.js` or custom readers. For plugin-declared formats, sibling files in the same folder resolve relative to the URL (such as `.mtl` beside `.obj`, or `.bin` and textures beside `.gltf`).
- **[`item.previewUrl`](../operations/item-files.md#item-previewurl)**: Returns a cached preview URL scaled to `size` pixels on its long edge across all visual formats, including camera RAW and PSD.
- **[`archive.entryUrl`](../operations/archive.md#archive-entryurl)**: Returns a URL for one member inside a ZIP, CBZ, EPUB, 7z or CB7 archive, optionally scaled to `size` pixels.

## Write URLs

When a plugin page creates binary output (from `<canvas>` or WebAssembly), call [`item.writeUrl`](../operations/item-files.md#item-writeurl) with mode `'replace'` or `'beside'`. Upload the bytes with `fetch(url, { method: 'PUT', body: blob })`:

- **`'replace'`**: Replaces the item's file in place (same file type; previous file moves to the Recycle Bin) and responds like [`item.replaceFile`](../operations/item-files.md#item-replacefile).
- **`'beside'`**: Creates a new sibling item inheriting the original item's folders, tags, rating, notes, source URL and custom fields, responding like [`item.add.beside`](../operations/item-add.md#item-add-beside).
- **Single-use scoping**: Each write URL works once and accepts writes only from the requesting plugin for that item.

## URL downloads

When calling [`item.add.fromUrls`](../operations/item-add.md#item-add-fromurls):

- **Candidate URLs (`alternates`)**: Pass fallback or higher-resolution candidate URLs so Curio tries preferred candidates before falling back.
- **Request headers (`headers`)**: Pass per-download headers such as `Referer`, `User-Agent` or `Cookie` for signed-in sites. Curio uses these headers only during the download and never stores them.
- **Local-network blocking**: Curio rejects loopback and private LAN addresses (`127.0.0.1`, `localhost`, RFC 1918 and link-local ranges) so web callers cannot probe local services.

## Queue & wait

All `item.add.*` operations accept an optional **`requestId`** (up to 128 characters) and a **`wait`** boolean:

- **Idempotency (`requestId`)**: Resending the same `requestId` within **10 minutes** returns the original result (or `{"success": true, "queued": true}` while running) with `"repeat": true`. Curio never imports the file twice.
- **Immediate vs waited responses**: With `wait: false`, or past the call timeout (60 s for URLs, 120 s for files), Curio returns `{"success": true, "queued": true}` and continues in its persistent queue.
- **Correlating finished items**: Subscribe to [`items.added`](../events/items.md#items-added) and match `event.data.requestId` to collect the created item UUIDs once the queued import completes.

## Duplicates

When importing bytes or downloading URLs, pass **`onDuplicate`** to control byte-identical (or bookmark-URL-identical) duplicates:

| Value | Behavior |
|---|---|
| `keep` *(default for direct calls)* | Imports the item anyway and resolves filename clashes using `onClash`. |
| `skip` *(default for Send to Curio)* | Skips adding if the library already holds identical file bytes (or the same bookmark URL). |
| `tell` | Skips adding and shows a desktop notification with a **Keep both** button. |
