# Runtime

Plugin programs are native executables or scripts (Python, Node.js, Rust, Go or shell scripts) that Curio spawns directly without an intermediate shell. Running any program requires the `exec` permission.

## Program types

| Program kind | Manifest key | Primary I/O contract |
|---|---|---|
| **Action** | `actions[].run` | JSON on `stdin` (`assets[]`), optional JSON `{ message, set }` on `stdout` |
| **URL handler** | `urls[].run` | JSON on `stdin` (`urls[]`, `place`), optional JSON `{ message, set }` on `stdout` |
| **Extractor** | `extractors[].command` | `{input}` and `{parts}` in argv, flat JSON `{ field: value }` on `stdout` |
| **Thumbnailer** | `formats[].thumbnail.command` | `{input}`, `{output}` and `{size}` in argv, writes image to `{output}` |
| **Managed job** | [`process.run`](../../api/operations/process.md#process-run) | Streamed `job.output`, isolated `CURIO_RUN_FOLDER`, cancellable (3,600 s default timeout, max 24 h) |

## Argv & cwd

- **Command array**: Specify the executable and arguments as a JSON array (`["python", "shapes.py"]` or `["bin/worker", "--fast"]`). Curio executes the command directly without invoking `cmd.exe` or `/bin/sh`.
- **Relative executables**: If the first element matches a file inside the plugin's folder, Curio resolves it to that absolute path; otherwise the OS searches `PATH`.
- **Working directory**: Every program starts in the plugin's folder, except `process.run`, which starts in its dedicated temporary `CURIO_RUN_FOLDER`.
- **Python bytecode guard**: Curio sets `PYTHONDONTWRITEBYTECODE=1` so Python scripts do not write `__pycache__` files inside library-local plugin folders and invalidate their per-computer approval hash.

## Environment

Every launched program receives three environment variables for live API access:

| Variable | Value |
|---|---|
| `CURIO_API_URL` | Loopback API base URL of the spawning Curio process (e.g. `http://127.0.0.1:52817/api/v1/`). |
| `CURIO_API_TOKEN` | Ephemeral single-run Bearer token scoped to this plugin's granted permissions and originating library. |
| `CURIO_PLUGIN` | Your plugin's `id`. |

Node.js programs using `curio.js` call `Curio.connect()` with no arguments; `curio.js` reads `CURIO_API_URL` and `CURIO_API_TOKEN` from `process.env`.

## Run tokens

- **Single-run lifetime**: Curio creates `CURIO_API_TOKEN` for one invocation and revokes it as soon as the child process exits.
- **Scope**: The endpoint behind `CURIO_API_URL` accepts program run tokens only (refusing browser origins), operates only on the originating library and enforces the permissions granted to the plugin in **Settings › Plugins** (`library.manage` is never granted).

## Stdio & timeouts

- **Exit code `0`**: Curio treats exit code `0` as success.
- **Non-zero exit code or timeout**: Curio treats the run as a failure and logs it. See [Loading and failures](../start/loading.md#failures) for per-kind timeout limits.
- **`stderr`**: Curio captures `stderr` and writes it to `curio.log` tagged with your plugin ID.
- **Process tree termination**: When a job times out or is cancelled, Curio terminates the child process along with any subprocesses it spawned.

## Failures

Consecutive program failures trigger Curio's failure circuit breaker, which disables the plugin until the user resolves the error and re-enables it. Inspect `curio.log` (and use [`plugin.log`](../../api/operations/plugin.md#plugin-log) for structured diagnostics), then click **Reload** in **Settings › Plugins** and re-enable the plugin. See [Loading and failures](../start/loading.md#failures) for circuit-breaker thresholds.
