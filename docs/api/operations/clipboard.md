# clipboard

Clipboard operations read and write plain text on the operating system clipboard and place library item files onto the clipboard for pasting into a file manager or external application. All three operations require the `ui` permission.

`clipboard.readText` and `clipboard.writeText` target the application as a whole and do not require an open library or window. `clipboard.copyFiles` targets a library to resolve item IDs to on-disk paths.

## clipboard.readText {#clipboard-readtext}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/clipboard/readText</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: clipboard_readText</span>
</div>

Read the current plain-text contents of the system clipboard.

**Parameters**

None. Passing `library` or `window` is rejected with `bad_request`.

**Returns**

`{ text: string | null }` — the clipboard text, or `null` if the clipboard is empty, holds non-text content or cannot be accessed.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/clipboard/readText \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const { text } = await curio.clipboard.readText()
    ```

=== "MCP"

    ```json
    {
      "name": "clipboard_readText",
      "arguments": {}
    }
    ```

=== "Plugin page"

    ```js
    const { text } = await curio.clipboard.readText()
    ```

=== "Program"

    ```js
    const { text } = await curio.clipboard.readText()
    ```

**Example response**

```json
{
  "text": "https://example.com/artwork/42"
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui`, or `bad_request` (400) if `library` or `window` is supplied.

## clipboard.writeText {#clipboard-writetext}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/clipboard/writeText</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: clipboard_writeText</span>
</div>

Replace the system clipboard contents with plain text.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `text` | `string` | Yes | — | Plain text to place on the clipboard. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/clipboard/writeText \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"text": "tag = landscape AND rating >= 4"}'
    ```

=== "JavaScript"

    ```js
    await curio.clipboard.writeText({ text: 'tag = landscape AND rating >= 4' })
    ```

=== "MCP"

    ```json
    {
      "name": "clipboard_writeText",
      "arguments": {
        "text": "tag = landscape AND rating >= 4"
      }
    }
    ```

=== "Plugin page"

    ```js
    await curio.clipboard.writeText({ text: promptText })
    ```

=== "Program"

    ```js
    await curio.clipboard.writeText({ text: shareUrl })
    ```

**Example response**

```json
{
  "success": true
}
```

**Events and errors**

Publishes no events. Returns `bad_request` (400) if `text` is missing or `library`/`window` is passed, `not_allowed` (403) without `ui` or `failed` (422) if the operating system clipboard cannot be written.

## clipboard.copyFiles {#clipboard-copyfiles}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/clipboard/copyFiles</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: clipboard_copyFiles</span>
</div>

Place the on-disk file paths of one or more library items onto the system clipboard's file list so they can be pasted directly into a file manager or another desktop app. Items whose files are missing or on offline storage are skipped as long as at least one file in the batch exists on disk.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `ids` | `(string \| number)[]` | Yes | — | Item UUIDs or internal row IDs whose files should be copied to the clipboard. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns**

`{ count: number }` — number of existing files placed on the clipboard.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/clipboard/copyFiles \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"ids": ["9d1e4f2a-6b8c-4a1d-9c2e-1a2b3c4d5e6f", "7c3b2a10-4e5f-4123-89ab-fedcba987654"]}'
    ```

=== "JavaScript"

    ```js
    const { count } = await curio.clipboard.copyFiles({
      ids: ['9d1e4f2a-6b8c-4a1d-9c2e-1a2b3c4d5e6f', '7c3b2a10-4e5f-4123-89ab-fedcba987654'],
    })
    ```

=== "MCP"

    ```json
    {
      "name": "clipboard_copyFiles",
      "arguments": {
        "ids": ["9d1e4f2a-6b8c-4a1d-9c2e-1a2b3c4d5e6f"]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { count } = await curio.clipboard.copyFiles({ ids: selectedIds })
    ```

=== "Program"

    ```js
    const { count } = await curio.clipboard.copyFiles({ ids: [itemId] })
    ```

**Example response**

```json
{
  "count": 2
}
```

**Events and errors**

Publishes no events. Returns `not_found` (404) if any item ID does not exist in the library or if none of the resolved items' files can be reached on disk, `not_allowed` (403) without `ui`, `library_closed` / `library_offline` (409) if the library is unavailable or `failed` (422) if the OS clipboard rejects the file list.
