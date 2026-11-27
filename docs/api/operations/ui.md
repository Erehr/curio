# ui

Interactive user-interface operations let scripts, MCP tools and plugins show notifications, prompt the user for input or choices in a Curio window, or draw menus and popovers outside a sandboxed plugin frame.

Window-targeted `ui.*` operations (`ui.notify`, `ui.confirm`, `ui.prompt`, `ui.pickTags`, `ui.pickFolders`, `ui.pickField`, `ui.pickColor` and `ui.menu`) require the `ui` permission and target a live Curio window selected by `window`, `library` or the most recently focused window (see [Choose a library or window](../connect/routing.md)):

- **Single prompt lock**: Interactive prompts (`ui.confirm` through `ui.menu`) bring the target window forward and allow only one active prompt per window at a time; concurrent callers receive `busy` (503).
- **Timeout**: Interactive prompts wait up to 10 minutes (600 seconds) for an answer before canceling the prompt and returning `busy` (503).
- **Plugin windows**: When called from a plugin page inside a secondary plugin window, Curio routes `ui.*` operations to that plugin window rather than the main window.

The two overlay operations (`ui.choose` and `ui.popover`) are answered directly by the host window over the plugin page bridge so menus and popovers can extend beyond the plugin iframe. They require only `library.read` and are available exclusively to plugin pages (see [Show controls outside a page frame](../../plugins/pages/overlays.md)).

Choosing between menu and overlay controls:

- **[`ui.menu`](#ui-menu)**: Context menu with icons and one-level submenus at the pointer; available to all callers with `ui` permission.
- **[`ui.choose`](#ui-choose)**: Native option list anchored below a control (with optional search and checkmarks); plugin pages only (`library.read`).
- **[`ui.popover`](#ui-popover)**: Floating HTML page from the same plugin anchored to a control for custom interactive layouts; plugin pages only (`library.read`).

## ui.notify {#ui-notify}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/notify</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_notify</span>
</div>

Show a transient toast notification in a Curio window. Unlike interactive prompts, `ui.notify` does not raise the window, does not wait for user dismissal and does not conflict with an open prompt dialog.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `text` | `string` | Yes | — | Notification message text. Maximum 2,000 characters. |
| `kind` | `'info' \| 'error'` | No | `'info'` | Visual severity of the toast. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library (ID, path or unique name). Not allowed for plugin callers. |

**Returns**

[`Done`](../types/runtime.md#done) — `{ success: true }`.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/notify \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"text": "Exported 12 items to ~/Exports", "kind": "info"}'
    ```

=== "JavaScript"

    ```js
    const curio = Curio.connect({ token: 'curio_your_token' })
    await curio.ui.notify({ text: 'Exported 12 items to ~/Exports', kind: 'info' })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_notify",
      "arguments": {
        "text": "Exported 12 items to ~/Exports",
        "kind": "info"
      }
    }
    ```

=== "Plugin page"

    ```js
    const curio = Curio.connect()
    await curio.ui.notify({ text: 'Conversion complete' })
    ```

=== "Program"

    ```js
    const curio = Curio.connect()
    await curio.ui.notify({ text: 'Could not reach remote host', kind: 'error' })
    ```

**Example response**

```json
{
  "success": true
}
```

**Events and errors**

Publishes no events. Returns `bad_request` (400) if `text` exceeds 2,000 characters or `kind` is neither `info` nor `error`, `not_allowed` (403) if the caller lacks `ui` (or a plugin specifies `window` or `library`), `no_window` (409) if no matching window is open or `busy` (503) if the window does not acknowledge within 15 seconds.

## ui.confirm {#ui-confirm}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/confirm</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_confirm</span>
</div>

Display a modal confirmation dialog in a Curio window and wait for the user to confirm or cancel. For external and plugin callers, the dialog displays the caller's token or plugin name so the user knows who is asking.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | Yes | — | Dialog header text. Maximum 200 characters. |
| `body` | `string` | No | — | Explanatory message shown above the buttons (whitespace preserved). Maximum 4,000 characters. |
| `okLabel` | `string` | No | Localized `OK` | Label for the confirming button. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ ok: boolean }` — `true` if the user clicked the confirm button, `false` if they canceled or closed the dialog.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/confirm \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Overwrite files?", "body": "3 files already exist in the export folder.", "okLabel": "Overwrite"}'
    ```

=== "JavaScript"

    ```js
    const { ok } = await curio.ui.confirm({
      title: 'Overwrite files?',
      body: '3 files already exist in the export folder.',
      okLabel: 'Overwrite',
    })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_confirm",
      "arguments": {
        "title": "Overwrite files?",
        "body": "3 files already exist in the export folder.",
        "okLabel": "Overwrite"
      }
    }
    ```

=== "Plugin page"

    ```js
    const { ok } = await curio.ui.confirm({ title: 'Reset plugin cache?', okLabel: 'Reset' })
    ```

=== "Program"

    ```js
    const { ok } = await curio.ui.confirm({ title: 'Continue batch upload?' })
    ```

**Example response**

```json
{
  "ok": true
}
```

**Events and errors**

Publishes no events. Returns `bad_request` (400) if `title` exceeds 200 characters or `body` exceeds 4,000 characters, `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if the window already has an active prompt/dialog or nobody answers within 600 seconds.

## ui.prompt {#ui-prompt}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/prompt</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_prompt</span>
</div>

Display a modal single-line text input dialog in a Curio window and wait for the user's entry. Pressing Enter submits the current value; canceling or closing the dialog returns `null`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | Yes | — | Dialog header text. Maximum 200 characters. |
| `body` | `string` | No | — | Explanatory message shown above the text field. Maximum 4,000 characters. |
| `value` | `string` | No | `""` | Initial text pre-filled in the input box. |
| `placeholder` | `string` | No | `""` | Placeholder hint shown when the input is empty. |
| `okLabel` | `string` | No | Localized `OK` | Label for the confirming button. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ value: string | null }` — the entered string (which may be `""` if submitted empty), or `null` if the user canceled or dismissed the dialog.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/prompt \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Collection name", "placeholder": "Summer 2026", "okLabel": "Create"}'
    ```

=== "JavaScript"

    ```js
    const { value } = await curio.ui.prompt({
      title: 'Collection name',
      placeholder: 'Summer 2026',
      okLabel: 'Create',
    })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_prompt",
      "arguments": {
        "title": "Collection name",
        "placeholder": "Summer 2026",
        "okLabel": "Create"
      }
    }
    ```

=== "Plugin page"

    ```js
    const { value } = await curio.ui.prompt({ title: 'Preset name', value: 'Default' })
    ```

=== "Program"

    ```js
    const { value } = await curio.ui.prompt({ title: 'API key for remote service' })
    ```

**Example response**

```json
{
  "value": "Summer 2026"
}
```

**Events and errors**

Publishes no events. Returns `bad_request` (400) if `title` exceeds 200 characters or `body` exceeds 4,000 characters, `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if another prompt is already open or the 600-second wait expires.

## ui.pickTags {#ui-picktags}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/pickTags</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_pickTags</span>
</div>

Open Curio's tag picker popover at the window's last pointer position so the user can toggle existing tags or create new ones. When the user closes the picker, the operation resolves with the list of selected tag names.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | — | Optional title accepted on the wire. Curio's inline tag picker popover does not render a separate header bar. |
| `selected` | `(string \| number)[]` | No | `[]` | Tags pre-selected when the picker opens, given by tag name or numeric tag ID. Names or IDs that do not exist in the library are ignored. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ tags: string[] | null }` — the names of all ticked tags when the picker closes, or `null` if canceled without committing. Timing out after 600 seconds cancels the prompt and returns `busy` (503), not `null`.

!!! note "Picker dismissal semantics"
    Because `ui.pickTags` uses Curio's live-toggle tag picker popover, closing the popover (clicking outside or pressing Escape) commits whatever tags are currently ticked and returns that array rather than `null`.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/pickTags \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"selected": ["landscape", "featured"]}'
    ```

=== "JavaScript"

    ```js
    const { tags } = await curio.ui.pickTags({ selected: ['landscape', 'featured'] })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_pickTags",
      "arguments": {
        "selected": ["landscape", "featured"]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { tags } = await curio.ui.pickTags({ selected: currentTags })
    ```

=== "Program"

    ```js
    const { tags } = await curio.ui.pickTags()
    ```

**Example response**

```json
{
  "tags": ["landscape", "featured", "sunset"]
}
```

**Events and errors**

Publishes `tags.changed` if the user creates a new tag inside the picker. Returns `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if the window already has an active prompt or the 600-second wait expires.

## ui.pickFolders {#ui-pickfolders}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/pickFolders</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_pickFolders</span>
</div>

Open Curio's folder picker popover at the window's last pointer position so the user can select one or more folders. Unlike `ui.pickTags`, every folder ID passed in `selected` is validated against the library before opening the picker.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | — | Optional title accepted on the wire. Curio's inline folder picker popover does not render a separate header bar. |
| `selected` | `number[]` | No | `[]` | Folder IDs pre-selected when the picker opens. Every ID must exist in the library. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ folders: number[] | null }` — the numeric IDs of all ticked folders when the picker closes, or `null` if canceled without committing. As with `ui.pickTags`, closing the live-toggle folder popover commits the currently ticked folders, while timing out after 600 seconds cancels the prompt and returns `busy` (503).

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/pickFolders \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"selected": [4, 12]}'
    ```

=== "JavaScript"

    ```js
    const { folders } = await curio.ui.pickFolders({ selected: [4, 12] })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_pickFolders",
      "arguments": {
        "selected": [4, 12]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { folders } = await curio.ui.pickFolders({ selected: [4] })
    ```

=== "Program"

    ```js
    const { folders } = await curio.ui.pickFolders()
    ```

**Example response**

```json
{
  "folders": [4, 12, 19]
}
```

**Events and errors**

Publishes no events. Returns `not_found` (404) if any ID in `selected` does not match an existing folder, `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if another prompt is active or the 600-second wait expires.

## ui.pickField {#ui-pickfield}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/pickField</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_pickField</span>
</div>

Display a searchable modal list of the library's custom fields and wait for the user to pick one. Clicking a row or pressing Enter/Space returns that field's key immediately.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | Localized default header | Dialog header text. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ key: string | null }` — the selected field's `key`, or `null` if canceled or dismissed.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/pickField \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Choose field to export"}'
    ```

=== "JavaScript"

    ```js
    const { key } = await curio.ui.pickField({ title: 'Choose field to export' })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_pickField",
      "arguments": {
        "title": "Choose field to export"
      }
    }
    ```

=== "Plugin page"

    ```js
    const { key } = await curio.ui.pickField({ title: 'Map column to field' })
    ```

=== "Program"

    ```js
    const { key } = await curio.ui.pickField()
    ```

**Example response**

```json
{
  "key": "camera_model"
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if another prompt is active or the 600-second wait expires.

## ui.pickColor {#ui-pickcolor}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/pickColor</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_pickColor</span>
</div>

Display a modal color picker dialog in a Curio window and wait for the user to choose a hex color.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | `string` | No | Localized default header | Dialog header text. |
| `value` | `string` | No | `'#808080'` | Starting color as `#rrggbb` (or 6 hex digits without `#`). Invalid strings fall back to `#808080`. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

**Returns**

`{ color: string | null }` — uppercase `#RRGGBB` hex string when confirmed, or `null` when canceled or dismissed.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/pickColor \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"title": "Accent color", "value": "#3B9160"}'
    ```

=== "JavaScript"

    ```js
    const { color } = await curio.ui.pickColor({ title: 'Accent color', value: '#3B9160' })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_pickColor",
      "arguments": {
        "title": "Accent color",
        "value": "#3B9160"
      }
    }
    ```

=== "Plugin page"

    ```js
    const { color } = await curio.ui.pickColor({ value: '#3B9160' })
    ```

=== "Program"

    ```js
    const { color } = await curio.ui.pickColor({ title: 'Highlight tint' })
    ```

**Example response**

```json
{
  "color": "#3B9160"
}
```

**Events and errors**

Publishes no events. Returns `not_allowed` (403) without `ui`, `no_window` (409) if no matching window is open or `busy` (503) if another prompt is active or the 600-second wait expires.

## ui.menu {#ui-menu}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/ui/menu</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Window, Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: ui_menu</span>
</div>

Show a context menu at the last pointer position in a Curio window and return the `id` of the chosen entry. When invoked from a plugin page, `curio.js` forwards the pointer's frame coordinates so the menu opens at the cursor inside the frame (or directly below a viewer toolbar button when invoked within 2 seconds of a right-click on that button).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `items` | `MenuItem[]` | Yes | — | Top-level menu entries. Total count across top-level entries and all submenu entries must be between 1 and 200. |
| `items[].id` | `string` | Conditional | — | Value returned when chosen. Required unless `separator: true` or the entry has non-empty `items`. |
| `items[].label` | `string` | Conditional | — | Entry text. Required unless `separator: true`. Maximum 200 characters. |
| `items[].icon` | `string` | No | — | Tabler icon name (for example `tag`, `folder`, `download`). |
| `items[].disabled` | `boolean` | No | `false` | Disables selection when `true`. |
| `items[].separator` | `boolean` | No | `false` | Renders a horizontal divider line; other fields on the entry are ignored. |
| `items[].items` | `MenuEntry[]` | No | `[]` | Submenu entries (one level deep; submenu entries cannot declare nested `items`). Each non-separator submenu entry requires both `id` and `label`. |
| `window` | `string` | No | Most recently focused window | Target window ID from [`library.list`](libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Default window's library | Target the window showing this library. Not allowed for plugin callers. |

Inline entry shapes:

```ts
type MenuEntry = { id?: string; label?: string; icon?: string; disabled?: boolean; separator?: boolean }
type MenuItem = MenuEntry & { items?: MenuEntry[] }
```

**Returns**

`{ id: string | null }` — the chosen entry's `id`, or `null` if the menu was dismissed without choosing.

**Example request**

=== "HTTP"

    ```bash
    curl -X POST http://127.0.0.1:41733/api/v1/ui/menu \
      -H "Authorization: Bearer curio_your_token" \
      -H "Content-Type: application/json" \
      -d '{"items": [{"id": "copy", "label": "Copy link", "icon": "link"}, {"separator": true}, {"label": "Export as", "icon": "download", "items": [{"id": "png", "label": "PNG"}, {"id": "webp", "label": "WebP"}]}]}'
    ```

=== "JavaScript"

    ```js
    const { id } = await curio.ui.menu({
      items: [
        { id: 'copy', label: 'Copy link', icon: 'link' },
        { separator: true },
        {
          label: 'Export as',
          icon: 'download',
          items: [
            { id: 'png', label: 'PNG' },
            { id: 'webp', label: 'WebP' },
          ],
        },
      ],
    })
    ```

=== "MCP"

    ```json
    {
      "name": "ui_menu",
      "arguments": {
        "items": [
          { "id": "copy", "label": "Copy link", "icon": "link" },
          { "separator": true },
          { "id": "archive", "label": "Archive selected", "icon": "archive" }
        ]
      }
    }
    ```

=== "Plugin page"

    ```js
    const { id } = await curio.ui.menu({
      items: [
        { id: 'fit', label: 'Fit to width', icon: 'arrows-horizontal' },
        { id: 'actual', label: 'Actual size', icon: 'zoom-reset' },
      ],
    })
    ```

=== "Program"

    ```js
    const { id } = await curio.ui.menu({
      items: [
        { id: 'skip', label: 'Skip existing' },
        { id: 'overwrite', label: 'Overwrite existing' },
      ],
    })
    ```

**Example response**

```json
{
  "id": "webp"
}
```

**Events and errors**

Publishes no events. Returns `bad_request` (400) if `items` is empty, the total number of entries (including submenus) exceeds 200, a non-separator entry lacks `label` (or lacks `id` without a submenu) or a `label` exceeds 200 characters; `not_allowed` (403) without `ui`; `no_window` (409) if no matching window is open; or `busy` (503) if another prompt is active or the 600-second wait expires.

## ui.choose {#ui-choose}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Open a native-styled option list anchored below a control in a plugin page, rendered by the host window outside the iframe so it is never clipped by the page's boundaries. Focus returns to the plugin iframe automatically when the list closes. Opening a second `ui.choose` resolves the previous one with `{ chosen: false }`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `options` | `(string \| number \| boolean \| ChooseOption)[]` | Yes | — | Non-empty array of primitive values or `ChooseOption` objects (truncated at 1,000 entries). Primitives become `{ value: entry, label: String(entry) }`. |
| `options[].value` | `string \| number \| boolean \| null` | No | `null` | Value returned when chosen (non-primitive values become `null`). |
| `options[].label` | `string` | No | `String(value ?? '')` | Display label (max 200 characters). |
| `options[].icon` | `string` | No | — | Tabler icon name (max 60 characters). |
| `options[].hint` | `string` | No | — | Right-aligned faint text (max 120 characters). |
| `options[].heading` | `boolean` | No | `false` | Renders a non-selectable section header when `true`. |
| `options[].disabled` | `boolean` | No | `false` | Disables selection when `true`. |
| `value` | `string \| number \| boolean \| null` | No | `null` | Value of the currently selected option, highlighted with a checkmark. |
| `search` | `boolean` | No | `false` | Adds a live filter box at the top of the list. |
| `at` | `{ x: number; y: number; w: number; h: number }` | No | `{ x: 0, y: 0, w: 0, h: 0 }` | Anchor rectangle in CSS pixels relative to the plugin page's viewport (typically from `element.getBoundingClientRect()`). |

Inline option shape:

```ts
type ChooseOption = { value?: string | number | boolean | null; label?: string; icon?: string; hint?: string; heading?: boolean; disabled?: boolean }
```

**Returns**

`{ chosen: boolean; value?: string | number | boolean | null }` — `{ chosen: true, value }` when the user picks an option, or `{ chosen: false }` when dismissed.

**Example request**

=== "Plugin page"

    ```js
    const curio = Curio.connect()
    const r = buttonEl.getBoundingClientRect()
    const pick = await curio.ui.choose({
      options: [
        { label: 'Reading mode', heading: true },
        { value: 'single', label: 'Single page', icon: 'file', hint: '1' },
        { value: 'spread', label: 'Two-page spread', icon: 'book-2', hint: '2' },
        { value: 'strip', label: 'Vertical strip', icon: 'arrows-vertical', hint: '3' },
      ],
      value: mode,
      search: false,
      at: { x: r.left, y: r.top, w: r.width, h: r.height },
    })
    if (pick.chosen) mode = pick.value
    ```

**Example response**

```json
{
  "chosen": true,
  "value": "spread"
}
```

**Events and errors**

Publishes no events. Returns `bad_request` if `options` is missing or empty, or if invoked outside a plugin page (HTTP, MCP or plugin program).

## ui.popover {#ui-popover}

<div class="curio-meta">
  <span class="curio-badge">Transport: Plugin page bridge</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Plugin pages only</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Open another HTML page from the same plugin in a floating popover anchored to a control in the calling page, rendered by the host window outside the calling iframe.

The popover page receives `data` via the [`plugin.run`](../../plugins/hooks/lifecycle.md) event, auto-sizes vertically from its rendered body height (capped to the window height) and closes itself with [`plugin.close`](plugin.md#plugin-close) to return a result. Pressing Escape, clicking outside the popover or opening another popover closes it with `{ result: null }`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `page` | `string` | Yes | — | Relative path to an HTML page inside the plugin's folder (max 300 characters). Absolute paths, URL schemes and `..` segments are rejected. |
| `data` | `unknown` | No | `null` | Initial payload delivered to the popover page as `plugin.run`. |
| `width` | `number` | No | `300` | Popover width in CSS pixels, clamped to `160`–`640`. |
| `at` | `{ x: number; y: number; w: number; h: number }` | No | `{ x: 0, y: 0, w: 0, h: 0 }` | Anchor rectangle in CSS pixels relative to the calling page's viewport. The window places the popover below `at`, flipping above if there is insufficient vertical space. |

!!! note "Optional initial height fallback"
    The host window bridge also accepts an optional `height` number (CSS pixels) as an initial height before the popover page reports its measured body height via `ResizeObserver` (defaulting to `200` px until measured), though `height` is not declared in the generated `curio.d.ts` schema.

**Returns**

`{ result: unknown }` — the `result` passed by the popover page to [`plugin.close`](plugin.md#plugin-close), or `{ result: null }` if dismissed without a result.

**Example request**

=== "Plugin page"

    ```js
    const curio = Curio.connect()
    const r = triggerEl.getBoundingClientRect()
    const { result } = await curio.ui.popover({
      page: 'popovers/preset.html',
      data: { selected: currentPreset },
      width: 340,
      at: { x: r.left, y: r.top, w: r.width, h: r.height },
    })
    if (result) applyPreset(result)
    ```

**Example response**

```json
{
  "result": {
    "preset": "high-contrast",
    "gamma": 1.15
  }
}
```

**Events and errors**

Delivers `plugin.run` to the popover page when mounted. Returns `bad_request` if `page` is empty, uses a URL scheme, starts with `/`, contains `..` or is invoked outside a plugin page.
