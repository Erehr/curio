# Jobs

Lifecycle, progress and live process-output events for long-running background tasks started by Curio (`duplicates.find`, `field.rescan`) or by plugins (`job.start`, `process.run`). All four events require `library.read`. Plugins only receive events for their own jobs; web tokens see all jobs.

Subscribe over HTTP SSE by sending `{"events":["job.*"]}` to `POST /api/v1/events`, or use `curio.on(...)` below. See [Subscribe to the event stream](../connect/events.md) for stream filtering, heartbeats, replay windows and reconnection.

## job.started {#job-started}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: App / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: job.*</span>
</div>

Emitted when a new background job starts.

**Payload** (`Job`): Initial [`Job`](../types/runtime.md#job) snapshot (`state: "running"`, `ended: null`).

```javascript
curio.on('job.started', (ev) => console.log('Started:', ev.data.id, ev.data.title))
```

**Observed event**

```json
{
  "id": "1763941600000-a1b2c3d4-50",
  "name": "job.started",
  "library": null,
  "at": 1763941600000,
  "data": { "id": "dup-8c21a4", "plugin": null, "title": "Finding duplicates", "done": 0, "total": 500, "text": "", "state": "running", "result": null, "error": null, "started": 1763941600000, "ended": null }
}
```

---

## job.progress {#job-progress}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: App / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: job.*</span>
</div>

Emitted when a job reports progress through [`job.update`](../operations/job.md#job-update) or internal worker ticks.

**Payload** (`{ job: string; plugin: string | null; done: number; total: number; text: string }`)

| Field | Type | Description |
|---|---|---|
| `job` | `string` | Job ID. |
| `plugin` | `string \| null` | Owning plugin ID, or `null`. |
| `done` | `number` | Completed units. |
| `total` | `number` | Total units (`0` if unknown). |
| `text` | `string` | Status text. |

```javascript
curio.on('job.progress', (ev) => console.log(`${ev.data.done}/${ev.data.total}: ${ev.data.text}`))
```

**Observed event**

```json
{
  "id": "1763941602500-a1b2c3d4-51",
  "name": "job.progress",
  "library": null,
  "at": 1763941602500,
  "data": { "job": "dup-8c21a4", "plugin": null, "done": 250, "total": 500, "text": "Comparing thumbnails" }
}
```

---

## job.output {#job-output}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Batched a few times / sec</span>
  <span class="curio-badge curio-badge--muted">Wildcard: job.*</span>
</div>

Emitted for [`process.run`](../operations/process.md#process-run) jobs. Each event delivers a batch of lines printed to `stdout` or `stderr`.

**Payload** (`{ job: string; plugin: string | null; pipe: 'stdout' | 'stderr'; lines: string[] }`)

| Field | Type | Description |
|---|---|---|
| `job` | `string` | Job ID. |
| `plugin` | `string \| null` | Owning plugin ID. |
| `pipe` | `'stdout' \| 'stderr'` | Source output stream. |
| `lines` | `string[]` | Lines collected since the previous batch. |

```javascript
curio.on('job.output', (ev) => {
  for (const line of ev.data.lines) console.log(`[${ev.data.pipe}] ${line}`)
})
```

**Observed event**

```json
{
  "id": "1763941603100-a1b2c3d4-52",
  "name": "job.output",
  "library": null,
  "at": 1763941603100,
  "data": { "job": "proc-7f20b1", "plugin": "com.example.dl", "pipe": "stdout", "lines": ["[download]  42.5% of 18.2MiB"] }
}
```

---

## job.finished {#job-finished}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: App / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: job.*</span>
</div>

Emitted when a job reaches a terminal state (`done`, `failed` or `cancelled`).

**Payload** (`Job`): Final [`Job`](../types/runtime.md#job) snapshot with `result`, `error` and `ended` populated.

```javascript
curio.on('job.finished', (ev) => {
  if (ev.data.state === 'done') console.log('Result:', ev.data.result)
  else console.error('Ended:', ev.data.state, ev.data.error)
})
```

**Observed event**

```json
{
  "id": "1763941612400-a1b2c3d4-53",
  "name": "job.finished",
  "library": null,
  "at": 1763941612400,
  "data": { "id": "dup-8c21a4", "plugin": null, "title": "Finding duplicates", "done": 500, "total": 500, "text": "", "state": "done", "result": { "groups": [] }, "error": null, "started": 1763941600000, "ended": 1763941612400 }
}
```
