# tool

Inspect external CLI tools discovered by Curio for plugin automation (`ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick`, `7z`).

## tool.list {#tool-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/tool/list</span>
  <span class="curio-badge">Permission: library.read (+ exec for path)</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: tool_list</span>
</div>

Return the status, detected version and homepage for every external CLI program Curio discovers. `Tool.path` is included only for callers holding the `exec` permission.

**Parameters** None (`{}`).

**Returns** [`Tool[]`](../types/runtime.md#tool).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/tool/list \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const tools = await curio.tool.list()
    const hasYtDlp = tools.some((t) => t.name === 'yt-dlp' && t.found)
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tool_list","arguments":{}}}
    ```

**Example response**

```json
[
  {
    "name": "ffmpeg",
    "title": "FFmpeg",
    "about": "Convert video, audio and animated images",
    "found": true,
    "version": "9.0.2",
    "homepage": "https://ffmpeg.org/download.html",
    "chosen": false
  },
  {
    "name": "yt-dlp",
    "title": "yt-dlp",
    "about": "Download video and audio from web pages",
    "found": false,
    "version": null,
    "homepage": "https://github.com/yt-dlp/yt-dlp",
    "chosen": false
  }
]
```

**Events and errors** Read-only. Returns `401 unauthorized` without a valid token, or `403 not_allowed` without `library.read`. See [Long-running jobs and tools](../../plugins/programs/jobs.md) to declare and run tools in plugins.
