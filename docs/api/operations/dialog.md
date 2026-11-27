# dialog

Native OS file and folder chooser dialogs let scripts, MCP tools and plugins ask the user to pick files or paths on disk. All three operations require the `ui` permission, bring the target Curio window forward, parent the OS dialog to that window and hold the window's single-prompt lock (`busy` if another prompt or dialog is already open). When invoked by a plugin page inside a secondary plugin window, the dialog is parented to that plugin window.

None of these operations read or write file contents themselves. To import chosen paths into the library or export items to a chosen folder, follow up with [`item.add.fromPaths`](item-add.md#item-add-frompaths) or [`item.export`](item-lifecycle.md#item-export) (which require `files.outside` in addition to `files.write` where applicable).

## dialog.openFile {#dialog-openfile}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/dialog/openFile</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: dialog_openFile</span>
</div>

Open the operating system's file picker dialog over a Curio window and return the chosen file path(s).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | System default | Dialog window title. |
| `multiple` | `boolean` | No | `false` | Allow selecting multiple files at once. |
| `filters` | `{ name: string; extensions: string[] }[]` | No | `[]` | File type filters shown in the dialog. Leading `.` characters are stripped from extensions (for example `["png", "jpg", "webp"]`). |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ paths: string[] | null }` — array of absolute filesystem paths (length 1 when `multiple` is `false`), or `null` if the user canceled the dialog.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/dialog/openFile \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Choose reference images", "multiple": true, "filters": [{"name": "Images", "extensions": ["png", "jpg", "webp"]}]}'
    ```

=== "JavaScript"

    ```js
    const { paths } = await curio.dialog.openFile({
      title: 'Choose reference images',
      multiple: true,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'webp'] }],
    })
    ```

=== "MCP"

    ```json
    {
      "name": "dialog_openFile",
      "arguments": {
        "title": "Choose reference images",
        "multiple": true,
        "filters": [{ "name": "Images", "extensions": ["png", "jpg", "webp"] }]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { paths } = await curio.dialog.openFile({
      title: 'Import LUT',
      filters: [{ name: '3D LUT', extensions: ['cube'] }],
    })
    ```

=== "Program"

    ```js
    const { paths } = await curio.dialog.openFile({ multiple: true })
    ```

**Example response**

```json
{
  "paths": [
    "C:\\Users\\alex\\Pictures\\ref_01.png",
    "C:\\Users\\alex\\Pictures\\ref_02.webp"
  ]
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui` (or if a plugin passes `window` or `library`), `no_window` (409) if no matching window is open or `busy` (503) if the target window already has an active prompt or dialog.

## dialog.openFolder {#dialog-openfolder}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/dialog/openFolder</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: dialog_openFolder</span>
</div>

Open the operating system's folder picker dialog over a Curio window and return the chosen directory path.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | System default | Dialog window title. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ path: string | null }` — absolute path of the chosen folder, or `null` if the user canceled.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/dialog/openFolder \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Choose export destination"}'
    ```

=== "JavaScript"

    ```js
    const { path } = await curio.dialog.openFolder({ title: 'Choose export destination' })
    ```

=== "MCP"

    ```json
    {
      "name": "dialog_openFolder",
      "arguments": {
        "title": "Choose export destination"
      }
    }
    ```

=== "Plugin page"

    ```js
    const { path } = await curio.dialog.openFolder({ title: 'Select output folder' })
    ```

=== "Program"

    ```js
    const { path } = await curio.dialog.openFolder({ title: 'Select watch folder' })
    ```

**Example response**

```json
{
  "path": "C:\\Users\\alex\\Exports\\portfolio"
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if the target window already has an active prompt or dialog.

## dialog.saveFile {#dialog-savefile}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/dialog/saveFile</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: dialog_saveFile</span>
</div>

Open the operating system's Save As dialog over a Curio window and return the chosen destination path. Curio does not create or write the file.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | System default | Dialog window title. |
| `fileName` | `string` | No | — | Suggested file name pre-filled in the save dialog. |
| `filters` | `{ name: string; extensions: string[] }[]` | No | `[]` | File type filters shown in the dialog. Leading `.` characters are stripped from extensions. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ path: string | null }` — absolute path chosen by the user, or `null` if canceled.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/dialog/saveFile \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Save report", "fileName": "catalog.csv", "filters": [{"name": "CSV", "extensions": ["csv"]}]}'
    ```

=== "JavaScript"

    ```js
    const { path } = await curio.dialog.saveFile({
      title: 'Save report',
      fileName: 'catalog.csv',
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    })
    ```

=== "MCP"

    ```json
    {
      "name": "dialog_saveFile",
      "arguments": {
        "title": "Save report",
        "fileName": "catalog.csv",
        "filters": [{ "name": "CSV", "extensions": ["csv"] }]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { path } = await curio.dialog.saveFile({
      fileName: 'palette.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    ```

=== "Program"

    ```js
    const { path } = await curio.dialog.saveFile({ fileName: 'export.zip' })
    ```

**Example response**

```json
{
  "path": "C:\\Users\\alex\\Documents\\catalog.csv"
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if the target window already has an active prompt or dialog.
