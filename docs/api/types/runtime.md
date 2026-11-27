# Runtime

Runtime types for an open window's active view, operation metadata, background jobs, child processes, external tools and acknowledgement responses.

## View {#view}

Snapshot of what a Curio window displays ([`view.get`](../operations/view.md#view-get), [`view.show`](../operations/view.md#view-show), [`view.setSearch`](../operations/view.md#view-setsearch), [`view.setSample`](../operations/view.md#view-setsample), [`view.setFilters`](../operations/view.md#view-setfilters), [`view.showWith`](../operations/view.md#view-showwith) and [`view.changed`](../events/windows.md#view-changed)):

| Property | Type | Description |
|---|---|---|
| `show` | `'all' \| 'favorites' \| 'uncategorized' \| 'untagged' \| 'recent' \| 'random' \| 'trash' \| 'missing' \| 'folder' \| 'smartFolder' \| 'tag' \| 'location' \| 'locationFolder' \| 'folders' \| 'locations'` | Active sidebar scope. |
| `id` | `number \| number[] \| null` | Numeric ID (or array of IDs for multi-selected folders, locations or tags), or `null` for global views. |
| `path` | `string \| null` | Relative directory path inside a location when `show === 'locationFolder'`. |
| `name` | `string` | Human-readable heading of the current view. |
| `search` | `string` | Current raw text in the search box. |
| `query` | `string` | Combined effective query from the search box and active Filter bar chips. |
| `count` | `number` | Number of items currently shown. |
| `view` | `'grid' \| 'tags' \| \`tab:${string}\`` | Main workspace mode (`grid` covers Grid, Masonry, Justified, List and Spectrum; `tags` is Tag Manager; `tab:<plugin>/<tab>` is a plugin tab). |
| `sample` | `{ text: string }` | Active font sample text (`""` means each font uses its default script sample). |

## Operation {#operation}

Descriptor returned in the array from [`app.operations`](../operations/app.md#app-operations):

```typescript
interface Operation {
  name: string;                    // canonical dotted name, e.g. "item.query"
  description: string;             // short summary
  permission: string;              // primary permission, e.g. "metadata.read"
  kind: 'read' | 'write' | 'window';
  params: object;                  // JSON Schema Draft-07 object
}
```

## Job {#job}

State of a long-running background task ([`job.list`](../operations/job.md#job-list), [`job.get`](../operations/job.md#job-get), [`job.started`](../events/jobs.md#job-started) and [`job.finished`](../events/jobs.md#job-finished)):

| Property | Type | Description |
|---|---|---|
| `id` | `string` | Job identifier. |
| `plugin` | `string \| null` | Owning plugin ID, or `null` for built-in jobs (such as `duplicates.find` or `field.rescan`). |
| `title` | `string` | Title shown in Curio's progress toast. |
| `done` | `number` | Completed units so far. |
| `total` | `number` | Total units (`0` when indeterminate). |
| `text` | `string` | Current status line. |
| `state` | `'running' \| 'done' \| 'failed' \| 'cancelled'` | Lifecycle state. |
| `result` | `unknown` | Payload passed to `job.finish` (`{ code, stdout, stderr, folder }` for `process.run`; `{ groups, omitted }` for `duplicates.find`). |
| `error` | `string \| null` | Error message if `state === 'failed'`. |
| `started` | `number` | Start timestamp (milliseconds since Unix epoch). |
| `ended` | `number \| null` | Completion timestamp (milliseconds since Unix epoch), or `null` while running. |

## ProcessRun {#processrun}

Return shape of [`process.run`](../operations/process.md#process-run). The optional output fields (`code`, `stdout`, `stderr` and `error`) appear only when called with `wait: true`:

| Property | Type | Description |
|---|---|---|
| `job` | `string` | Job ID tracking the child process. |
| `folder` | `string` | Isolated temporary working directory (`CURIO_RUN_FOLDER`) created for this run. |
| `state` | `'running' \| 'done' \| 'failed' \| 'cancelled'` | `'running'` when called asynchronously, or the terminal state when `wait: true`. |
| `code` | `number \| null \| undefined` | Process exit code. |
| `stdout` | `string \| undefined` | Captured stdout (last 1 MiB). |
| `stderr` | `string \| undefined` | Captured stderr (last 1 MiB). |
| `error` | `string \| null \| undefined` | Launch or termination error message. |

## Tool {#tool}

An external CLI tool discovered by Curio ([`tool.list`](../operations/tool.md#tool-list)):

| Property | Type | Description |
|---|---|---|
| `name` | `string` | Canonical tool identifier (`ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick`, `7z`). |
| `title` | `string` | Display name (e.g. `"FFmpeg"`). |
| `about` | `string` | Short description of what the tool does. |
| `found` | `boolean` | Whether a working executable was located. |
| `version` | `string \| null` | Detected version string, or `null`. |
| `homepage` | `string` | Official download URL. |
| `chosen` | `boolean` | `true` if the user picked a custom path in **Settings › Plugins › Tools**. |
| `path` | `string \| null \| undefined` | Resolved executable path. Included **only** when the caller holds `exec`. |

## Done {#done}

Standard acknowledgement returned by mutations that produce no additional payload:

```typescript
type Done = { success: true };
```
