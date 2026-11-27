# process

Plugin-exclusive operation for spawning a bundled plugin executable or a declared system CLI tool (`ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick`, `7z`) inside an isolated temporary working directory tracked as a cancelable job.

## process.run {#process-run}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token</span>
  <span class="curio-badge">Permission: exec</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages & programs</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Launch a plugin executable (`program`) or a discovered external tool declared in `plugin.json`'s `tools[]` (`tool`) as a tracked job. The process runs inside a fresh temporary folder (`CURIO_RUN_FOLDER`) and receives `CURIO_API_URL`, `CURIO_API_TOKEN`, `CURIO_PLUGIN`, `CURIO_JOB` and `CURIO_RUN_FOLDER` in its environment. Files written inside `CURIO_RUN_FOLDER` can be imported via `item.add.fromPaths`, `item.replaceFile`, `item.add.beside` or `item.setThumbnail` **without** requiring `files.outside`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `tool` | `string` | One of `tool` / `program` | — | Discovered tool declared in `plugin.json`'s `tools[]`: `ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick` or `7z`. |
| `args` | `string[]` | No | `[]` | Arguments passed to `tool`. |
| `program` | `string[]` | One of `tool` / `program` | — | Command vector (`["bin/worker", "--fast"]`); relative first element resolves inside the plugin folder, otherwise resolved via `PATH`. |
| `stdin` | `string` | No | — | UTF-8 text piped to the process's standard input. |
| `title` | `string` | No | Program/tool name | Title shown in Curio's progress toast. |
| `timeout` | `number` | No | `3600` (1 hour) | Max execution time in seconds (up to `86400`, 24 hours). |
| `wait` | `boolean` | No | `false` | When `true`, waits up to 120 s and includes `code`, `stdout`, `stderr` and `error` directly in the response. |

**Returns** [`ProcessRun`](../types/runtime.md#processrun) (`{ job, folder, state, code?, stdout?, stderr?, error? }`).

**Example request**

=== "Plugin page (declared tool)"

    ```javascript
    const run = await curio.process.run({
      tool: 'ffprobe',
      args: ['-v', 'quiet', '-print_format', 'json', '-show_streams', item.path],
      title: 'Inspecting streams',
      wait: true,
    })
    if (run.code === 0) {
      const info = JSON.parse(run.stdout)
      console.log(info.streams)
    }
    ```

=== "Plugin page (streaming async)"

    ```javascript
    const run = await curio.process.run({
      tool: 'yt-dlp',
      args: ['-o', '%(title)s.%(ext)s', 'https://example.com/video'],
      title: 'Downloading video',
      wait: false,
    })
    const sub = curio.subscribe({ events: ['job.*'] }, (ev) => {
      if (ev.name === 'job.output' && ev.data.job === run.job) console.log(ev.data.lines)
      if (ev.name === 'job.finished' && ev.data.id === run.job) sub.close()
    })
    ```

**Example response**

```json
{
  "job": "proc-7f20b1",
  "folder": "C:\\Users\\You\\AppData\\Local\\Temp\\curio-run-7f20b1",
  "state": "done",
  "code": 0,
  "stdout": "{\"streams\":[...]}",
  "stderr": "",
  "error": null
}
```

**Events and errors** Emits [`job.started`](../events/jobs.md#job-started), [`job.output`](../events/jobs.md#job-output) (batched stdout/stderr lines) and [`job.finished`](../events/jobs.md#job-finished). Canceling via `job.cancel` kills the child process tree. Returns `400 bad_request` if neither or both of `program` and `tool` are provided or `tool` is unknown; `403 not_allowed` outside plugin callers, without `exec` permission or if `tool` is not listed in `plugin.json`'s `tools[]`; `404 not_found` if the tool is not installed on this computer; or `422 failed` if the temporary run folder cannot be created.
