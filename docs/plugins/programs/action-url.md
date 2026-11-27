# Actions & URLs

Action programs (`actions[].run`) and URL handler programs (`urls[].run`) receive a JSON envelope on `stdin`, can call Curio's live API while running and may return an optional JSON summary or update object on `stdout`. Both require the `exec` permission and run for up to **600 seconds** (10 minutes).

## Action input

When invoked from the item menu, toolbar, viewer, shortcut or a reusable action step, Curio writes JSON to `stdin` and closes `stdin`:

```json
{
  "api": 1,
  "event": "action",
  "action": "tag-shape",
  "assets": [
    {
      "uid": "0190f5e2-7c4b-7a31-9d2e-5b8c41f07a6e",
      "path": "D:\\Art\\a.png",
      "name": "a.png",
      "display_name": "",
      "ext": "png",
      "kind": "image",
      "size": 182733,
      "width": 832,
      "height": 1216,
      "rating": 4,
      "annotation": "",
      "source_url": "",
      "tags": ["sky"],
      "folders": ["Renders"],
      "palette": [{ "color": [52, 90, 160], "ratio": 41.2 }],
      "fields": { "model": "Anima", "steps": 12, "cfg": 1 }
    }
  ]
}
```

## URL input

When a URL matches `urls[].patterns` and the handler specifies `run`, Curio writes:

```json
{
  "api": 1,
  "event": "url",
  "handler": "video-dl",
  "urls": ["https://www.youtube.com/watch?v=dQw4w9WgXcQ"],
  "place": {
    "folders": [3],
    "tags": ["tutorials"],
    "annotation": null,
    "sourceUrl": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    "rating": null,
    "fields": null
  },
  "assets": []
}
```

## Item record {#asset-record}

Each entry in `assets[]` represents one item in the `curio-metadata/1` export shape, redacted by the plugin's granted permissions:

- **Without `metadata.read`**: Each entry contains only `uid` (plus `path` when `files.read` is granted).
- **Without `files.read`**: `path` is `null`.

## Return payload

Programs may print nothing to `stdout` (empty output is valid) or print one JSON object:

```json
{
  "message": "Tagged 4 portrait pictures",
  "set": [
    {
      "id": "0190f5e2-7c4b-7a31-9d2e-5b8c41f07a6e",
      "tags": ["portrait"],
      "rating": 5,
      "annotation": "Reviewed by script",
      "fields": { "review": "approved" }
    }
  ]
}
```

- **`message`**: Optional string Curio displays as a toast notification in the window.
- **`set`**: Optional array of metadata edits Curio applies without HTTP calls (requires `metadata.write`). Curio adds `tags` and sets `rating`, `annotation` and `fields`.

## Validation

- Curio validates every `set` entry against field types, choice `options` and `validation` bounds.
- A plugin may write only fields it owns (or shares) and user-defined fields, never built-in fields or another plugin's fields.
- An action program's `set` entries may reference only item IDs present in its `assets` input. A URL handler receives no initial items and may reference any item ID.

## Live API calls

For workflows such as downloading files, querying related items, creating folders or importing new files via `item.add.fromPaths`, call Curio directly using `CURIO_API_URL` and `CURIO_API_TOKEN`:

=== "Python"

    ```python
    import json, os, sys, urllib.request

    def call(op, params):
        req = urllib.request.Request(
            os.environ["CURIO_API_URL"] + op.replace(".", "/"),
            json.dumps(params).encode(),
            {
                "Authorization": "Bearer " + os.environ["CURIO_API_TOKEN"],
                "Content-Type": "application/json",
            },
        )
        with urllib.request.urlopen(req) as r:
            return json.load(r)

    payload = json.load(sys.stdin)
    ids = [a["uid"] for a in payload["assets"]]
    items = call("item.getMany", {"ids": ids})
    tall = [i["id"] for i in items if (i.get("height") or 0) > (i.get("width") or 0)]
    if tall:
        call("item.addTags", {"ids": tall, "tags": ["portrait"]})
    json.dump({"message": f"Tagged {len(tall)} portrait items"}, sys.stdout)
    ```

=== "Node.js"

    Download `curio.js` from `http://127.0.0.1:41733/api/v1/curio.js` or bundle it alongside your script.

    ```javascript
    const fs = require('node:fs')
    const Curio = require('./curio.js')

    async function main() {
      const input = JSON.parse(fs.readFileSync(0, 'utf8'))
      const curio = Curio.connect() // reads CURIO_API_URL & CURIO_API_TOKEN
      const ids = input.assets.map((a) => a.uid)
      await curio.item.addTags({ ids, tags: ['processed'] })
      process.stdout.write(JSON.stringify({ message: `Processed ${ids.length} items` }))
    }
    main()
    ```
