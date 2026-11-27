# URLs

URL handlers (`urls[]`) intercept matching web addresses when a user drops a link into Curio, pastes URLs with ++ctrl+v++, sends a link from the **Send to Curio** browser extension or calls [`item.add.fromUrls`](../../api/operations/item-add.md#item-add-fromurls).

See [Action and URL handler declarations](../manifest/actions.md#urls-id) for every key and default.

## Patterns

Declare URL matchers in `plugin.json`:

```json
{
  "urls": [
    {
      "id": "video-dl",
      "title": "Download video",
      "patterns": [
        "https://www.youtube.com/watch*",
        "https://youtu.be/*",
        "https://vimeo.com/*"
      ],
      "opens": "downloader"
    }
  ]
}
```

- **Whole-URL patterns (`patterns`)**: Each pattern matches the entire URL, with `*` matching any run of characters.
- **Mixed batches**: When multiple URLs arrive in one batch, Curio sends matching URLs to their handlers and routes non-matching URLs through the built-in downloader and bookmark pipeline.

## Entry points

URL handlers activate across every URL ingestion channel:

- Dragging a URL from a browser onto Curio's window or a sidebar folder
- Pasting one or more URLs with ++ctrl+v++
- Saving a page or stream link (`as: "link"`) from the **Send to Curio** browser extension
- Calling [`item.add.fromUrls`](../../api/operations/item-add.md#item-add-fromurls) unless `handlers: false` is passed

## Receivers

Each `urls[]` entry specifies either **`opens`** or **`run`**:

- **Page receiver (`opens`)**: Opens your plugin window (`"<windowId>"`) or tab (`"tab:<tabId>"`), reusing an existing single-instance window, and sends `{ handler, urls, place }` via [`plugin.run`](../hooks/lifecycle.md#plugin-run). Use a page receiver when the user needs to choose quality, format or files before downloading.
- **Program receiver (`run`)**: Requires `exec`. Runs in the background with `{ api: 1, event: "url", handler, urls, place, assets: [] }` on `stdin`. Use a program receiver for non-interactive downloaders such as `yt-dlp` or `gallery-dl`.

## Place payload

Both page and program receivers receive a `place` object containing the destination metadata chosen by the user or browser extension:

```json
{
  "folders": [3],
  "tags": ["tutorials", "video"],
  "annotation": "Blender lighting breakdown",
  "sourceUrl": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
  "rating": 4,
  "fields": { "topic": "Lighting" }
}
```

Pass these properties to `item.add.fromPaths` or `item.add.fromUrls` when importing downloaded files so Curio preserves the user's chosen folders, tags, notes, rating and custom fields.

## Dispatch

- **API reporting**: When `item.add.fromUrls` hands URLs to a plugin, its [`Added`](../../api/types/item.md#added) response includes `"handed": [{ "url": "...", "plugin": "<id>", "handler": "<handlerId>" }]`.
- **Recursion prevention**: Curio never routes URLs sent to `item.add.fromUrls` by a plugin back into that same plugin's URL handlers.
- **Explicit bypass**: Any caller can pass `"handlers": false` to `item.add.fromUrls` to run Curio's built-in downloader and bookmark handler directly.

## Background

- **Program handlers (`run`)**: Run even when a library is open only in a background holder (for example when Send to Curio saves into an unopened library).
- **Page handlers (`opens`)**: Require an open desktop window to display their UI. If a library may run in the background without a window, provide a headless `run` program or check window availability.
