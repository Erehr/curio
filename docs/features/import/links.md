# Links

Add remote images, downloadable files and web bookmarks to your library by pasting addresses, dropping links from a browser or calling [`item.add.fromUrls`](../../api/operations/item-add.md#item-add-fromurls) and [`item.add.bookmark`](../../api/operations/item-add.md#item-add-bookmark).

## Paste URLs

Paste a single `http://` or `https://` URL with ++ctrl+v++, or drag a link from a browser onto the grid or a sidebar folder. Use **+ › Import from links** to paste multiple URLs (one per line):

- Each valid URL is processed in order and placed in the open folder or the folder you dropped onto.
- Curio refuses non-HTTP schemes and loopback or private network addresses (`127.0.0.1`, `localhost` and LAN IPs). The downloader fetches only from the public internet.

## Download vs bookmark

When Curio receives a URL without an explicit bookmark choice, it inspects the response headers and content type:

| Remote target | Default behavior | Resulting asset |
|---|---|---|
| **Direct image, audio, video, document, archive, font or other file** | Downloads the file into `<Library>.curio/files/` | Local asset with its source URL preserved. |
| **Web page (`text/html`) or streaming video page** | Saves a `.url` bookmark file | Link asset pointing to the page, with its title and preview thumbnail. |
| **Explicit bookmark** (**Save as link** or `item.add.bookmark`) | Always saves a `.url` link regardless of target MIME type | Link asset; never downloads the remote file. |

## Previews

Every downloaded file and saved bookmark stores its origin in the **URL** field in the Inspector so you can trace where an asset came from or reopen the original page:

- **Page previews**: For saved web links, Curio creates a 512 px thumbnail (`512 × 288` or `512 × 320`) from Open Graph or poster metadata, or a page preview, according to **Settings › Preview › Link previews**.
- **YouTube and media links**: YouTube links fetch video titles and poster art automatically. View them inline or open them in your browser; see [Read text, code and web links](../viewer/text-links.md).
- **Browser-captured thumbnails**: When you save a page with the [Send to Curio](browser.md) browser extension, the extension attaches a screenshot of your browser tab as the thumbnail. Logged-in pages get an accurate preview without Curio making a second request.

## URL handlers

Before Curio downloads a URL or saves it as a bookmark, it checks enabled plugins for matching **URL handlers**:

- If an enabled plugin matches the address pattern (for example a `yt-dlp` downloader or gallery importer), Curio hands the URL to that plugin along with the target folder, tags, rating, notes, source URL and custom fields.
- External API callers can bypass plugin handlers for a specific call in [`item.add.fromUrls`](../../api/operations/item-add.md#item-add-fromurls).
- See [Register URL handlers](../../plugins/contributions/urls.md) to build your own handler.

## Failures & retry

Curio downloads remote URLs through a persistent download queue that survives browser closes and Curio restarts:

- **Duplicate checking**: When duplicate skipping is enabled (the default in the browser extension), Curio compares downloaded bytes against existing assets and bookmark URLs against existing links. Duplicates are skipped, and the notification offers a **Keep both** button if you want the copy anyway.
- **Alternate URLs**: Callers such as Send to Curio can pass higher-resolution candidate URLs (for example Wikimedia originals or full-size X images). Curio tries the largest candidate first and falls back to the displayed URL if the host refuses it.
- **Failed downloads**: Network errors, HTTP 4xx/5xx responses and timeouts show a toast (and a system notification if enabled in **Settings › Notifications**). Click **Retry** to try again without re-copying the link.
