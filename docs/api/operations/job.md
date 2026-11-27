# job

Operations for creating, updating, completing, canceling and inspecting long-running background jobs shown in Curio's progress toasts. All operations target the **application** (`Target: App`) and require `library.read`. Mutating job operations (`job.start`, `job.update`, `job.finish`, `job.cancel`) are restricted to plugins and the desktop host; `job.list` and `job.get` are shared across web, MCP and plugin callers.

## job.start {#job-start}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token / Host</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin, Window</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Register a new tracked background job shown in Curio's progress toast area with a Cancel button.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | Yes | — | Job title displayed to the user. |
| `total` | `number` | No | `0` | Total work units (`0` for indeterminate spinner). |

**Returns** `{ job: string }`.

**Example request**

=== "Plugin page"

    ```javascript
    const { job } = await curio.job.start({ title: 'Converting 12 images', total: 12 })
    ```

**Example response**

```json
{ "job": "job-9a14c2" }
```

**Events and errors** Emits [`job.started`](../events/jobs.md#job-started). Returns `403 not_allowed` outside plugin or window callers.

---

## job.update {#job-update}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token / Host</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin, Window</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Update progress counters and status text on a running job, and check whether the user clicked Cancel.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `job` | `string` | Yes | — | Job ID from `job.start`. |
| `done` | `number` | No | — | Completed units so far. |
| `total` | `number` | No | — | Updated total units. |
| `text` | `string` | No | — | Status line text. |

**Returns** `{ cancelled: boolean }`. Stop working when `cancelled === true`.

**Example request**

=== "Plugin page"

    ```javascript
    const { cancelled } = await curio.job.update({ job, done: 5, text: 'hero_05.png' })
    if (cancelled) return
    ```

**Example response**

```json
{ "cancelled": false }
```

**Events and errors** Emits [`job.progress`](../events/jobs.md#job-progress). Returns `403 not_allowed` outside plugin or window callers, or `404 not_found` if `job` does not exist or belongs to another plugin.

---

## job.finish {#job-finish}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token / Host</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin, Window</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Mark a job complete with an optional `result` payload (retained for 10 minutes), or failed with an `error` message.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `job` | `string` | Yes | — | Job ID. |
| `result` | `unknown` | No | `null` | Structured completion payload retrievable via `job.get`. |
| `error` | `string` | No | — | Non-empty error message marks the job `failed`. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.job.finish({ job, result: { converted: 12 } })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`job.finished`](../events/jobs.md#job-finished). Returns `403 not_allowed` outside plugin or window callers, or `404 not_found` if `job` does not exist or belongs to another plugin.

---

## job.cancel {#job-cancel}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page / Run token / Host</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin, Window</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Cancel a running job. If the job was started by [`process.run`](process.md#process-run), Curio terminates the child process and its entire process tree immediately.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `job` | `string` | Yes | — | Job ID to cancel. |

**Returns** [`Done`](../types/runtime.md#done).

**Example request**

=== "Plugin page"

    ```javascript
    await curio.job.cancel({ job })
    ```

**Example response**

```json
{ "success": true }
```

**Events and errors** Emits [`job.finished`](../events/jobs.md#job-finished) (`state: "cancelled"`). Returns `403 not_allowed` outside plugin or window callers, or `404 not_found` if `job` does not exist or belongs to another plugin.

---

## job.list {#job-list}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/job/list</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: job_list</span>
</div>

List all currently running jobs plus jobs that ended within the last **10 minutes**. Plugins see only their own jobs; web/MCP callers see all jobs.

**Parameters** None (`{}`).

**Returns** [`Job[]`](../types/runtime.md#job).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/job/list \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{}'
    ```

=== "JavaScript"

    ```javascript
    const jobs = await curio.job.list()
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"job_list","arguments":{}}}
    ```

**Example response**

```json
[
  { "id": "dup-8c21a4", "plugin": null, "title": "Finding duplicates", "done": 140, "total": 500, "text": "Hashing files", "state": "running", "result": null, "error": null, "started": 1763941000000, "ended": null }
]
```

**Events and errors** Read-only. Returns `401 unauthorized` without a valid token or `403 not_allowed` without `library.read`.

---

## job.get {#job-get}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/job/get</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: job_get</span>
</div>

Inspect one job's live progress or final `result`/`error` (available for 10 minutes after completion).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `job` | `string` | Yes | — | Job ID. |

**Returns** [`Job`](../types/runtime.md#job).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/job/get \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" -d '{"job":"dup-8c21a4"}'
    ```

=== "JavaScript"

    ```javascript
    const status = await curio.job.get({ job: 'dup-8c21a4' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"job_get","arguments":{"job":"dup-8c21a4"}}}
    ```

**Example response**

```json
{ "id": "dup-8c21a4", "plugin": null, "title": "Finding duplicates", "done": 500, "total": 500, "text": "", "state": "done", "result": { "groups": [] }, "error": null, "started": 1763941000000, "ended": 1763941012400 }
```

**Events and errors** Read-only. Returns `401 unauthorized` without a valid token, `403 not_allowed` without `library.read` or `404 not_found` if the job does not exist, belongs to another plugin or has expired.
