# Loading

Curio loads built-in format plugins first and then external plugins in folder order, keeping the first plugin for each ID and disabling plugins whose dependencies are missing or that fail repeatedly.

## Built-in

Curio includes 11 built-in format plugins compiled into the application (badged **core** in **Settings › Plugins**): `builtin.format.image`, `builtin.format.video`, `builtin.format.audio`, `builtin.format.document`, `builtin.format.code`, `builtin.format.link`, `builtin.format.design`, `builtin.format.archive`, `builtin.format.font`, `builtin.format.model` and `builtin.format.scene`. Built-in plugins stay always on and provide baseline format recognition, viewers and file readers.

## Discovery

External plugins load after built-in plugins in this order:

1. **User-added plugin folders**: Plugin directories (each containing `plugin.json`) registered via **Settings › Plugins › Add plugin** and stored in `app.json`. If a linked folder is moved, renamed or deleted, Curio removes the missing path on open or reload.
2. **Application plugins directory**: `%APPDATA%\Curio\plugins\<folder>\plugin.json` (Windows), `~/.local/share/Curio/plugins/<folder>/plugin.json` (Linux), `~/Library/Application Support/Curio/plugins/<folder>/plugin.json` (macOS) or `Curio data\plugins\` (portable mode). Available to every library on the computer.
3. **Per-library plugins directory**: `<Library>.curio/plugins/<folder>/plugin.json`. Travels with the `.curio` library folder and requires per-computer approval before running on a new machine.

## Validation

- **ID collisions**: Every plugin must declare a unique `id` (reverse-domain or dot-separated lowercase identifier, such as `com.example.tool` or `curio.reader`; `curio` is reserved). If two plugins share an `id`, Curio keeps the first one loaded in the discovery order above and marks later duplicates inactive with a problem banner.
- **`api` and `min_curio`**: `api` must not exceed Curio's supported manifest API version (`1`), and `min_curio` (if set) must be satisfied by the running Curio version. Plugins that fail either check load as inactive with a problem banner.
- **`dependencies`**: Map of required plugin IDs to version constraints (`{ "curio.reader": ">=0.7" }`). Curio resolves dependencies iteratively until the active set stabilizes; any plugin whose dependency is missing, inactive or out of range stays inactive with an explicit message in **Settings › Plugins**.
- **Format overrides**: When an active external plugin declares an extension already handled by a built-in format plugin, the external plugin overrides the built-in viewer and thumbnailer while enabled. **Settings › Plugins** displays an override notice on the overridden built-in entry.

## Approvals

- **Per-library state**: Enablement and granted permissions are stored inside each library's SQLite database (`plugin_state`), so different libraries can enable different plugins and grants.
- **Library-local approval**: Plugins inside `<Library>.curio/plugins/` stay paused on any new computer (or after any file inside the plugin folder changes) until approved by the user. See [Permissions](permissions.md#reapproval) for how content hashing and reapproval work.

## Failures

Clicking **Reload** in **Settings › Plugins** re-reads manifests, re-evaluates format overrides and refreshes fields, sidebars and the Inspector.

Every background runner enforces a hard timeout:

| Runner | Manifest or API key | Timeout |
|---|---|---|
| Command extractor | `extractors[].command` | 30 s |
| Hidden page worker | `formats[].thumbnail_page`, `extractors[].page` | 60 s |
| Command thumbnailer | `formats[].thumbnail.command` | 90 s |
| Action or URL program | `actions[].run`, `urls[].run` | 600 s (10 min) |
| Managed job | `process.run` | 3,600 s default (max 24 h; `wait: true` waits up to 120 s) |

**Consecutive failure circuit breaker**: If a plugin's program, extractor page or thumbnail page fails **5 times in a row** (non-zero exit code, crash or timeout), Curio disables the plugin and logs the failure. Fix the cause, click **Reload** and re-enable the plugin in **Settings › Plugins** to reset the counter. A single successful run also resets the counter.

## Retained data

| Data | When disabled or unlinked |
|---|---|
| Plugin files on disk | Never deleted by Curio (unlinking removes only the config entry). |
| Custom field definitions & extracted values | Stay readable in SQLite; inspect and remove orphaned values via **Settings › Fields › Clean up** ([`field.cleanup`](../../api/operations/field.md#field-cleanup)). |
| Private `plugin.state` & `plugin.itemState` | Retained in SQLite so re-enabling the plugin restores bookmarks, reading positions and saved state. |
| Computer-level `plugin.settings` | Retained in Curio's app data directory. |
