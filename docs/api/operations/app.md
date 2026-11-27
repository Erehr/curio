# app

Operations in the `app` namespace inspect the running Curio instance, discover permitted operations, list installed system fonts, read a window's active theme tokens and open external URLs in the system browser after user confirmation.

## app.info {#app-info}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/app/info</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: app_info</span>
</div>

Returns Curio's application name, version, API version, current display language, operating system platform and CPU architecture. Because this operation targets the application as a whole, passing `library` or `window` is rejected with `bad_request`.

**Parameters**

This operation takes no parameters (`{}`).

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `name` | `string` | Always `"Curio"`. |
| `version` | `string` | Curio's version string (for example `"1.0.0"`). |
| `apiVersion` | `number` | Numeric API protocol version (currently `1`). |
| `platform` | `'windows' \| 'linux' \| 'macos'` | Operating system identifier (`"windows"`, `"linux"` or `"macos"`). |
| `arch` | `string` | CPU architecture (for example `"x86_64"` or `"aarch64"`). |
| `language` | `string` | Active UI locale code (for example `"en"`, `"de"` or `"pt-BR"`). |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/app/info \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const info = await curio.app.info();
    console.log(info.version, info.platform, info.language);
    ```

=== "MCP"

    ```json
    {
      "name": "app_info",
      "arguments": {}
    }
    ```

**Example response**

```json
{
  "name": "Curio",
  "version": "1.0.0",
  "apiVersion": 1,
  "platform": "windows",
  "arch": "x86_64",
  "language": "en"
}
```

**Side effects, events and errors**

- Emits no public events and modifies no state.
- Errors: `unauthorized` (missing or invalid bearer token), `not_allowed` (missing `library.read`), `bad_request` (if `library` or `window` is passed).
- Related: [`app.operations`](#app-operations), [`library.list`](./libraries.md#library-list), [Web API and service discovery](../connect/http.md).

---

## app.operations {#app-operations}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/app/operations</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: app_operations</span>
</div>

Returns the list of operations that the current caller is permitted to invoke, including each operation's description, required permission, kind and JSON Schema parameter definition (including routing parameters `library` and `window` where applicable). Operations unavailable to the caller's kind or requiring ungranted permissions are omitted.

**Parameters**

This operation takes no parameters (`{}`).

**Returns**

[`Operation[]`](../types/runtime.md#operation) — an array of operation descriptors:

| Field | Type | Description |
|---|---|---|
| `name` | `string` | Dot-delimited operation name (for example `"item.query"`). |
| `description` | `string` | Human-readable summary of what the operation does. |
| `permission` | `string` | Primary permission required (for example `"metadata.read"`). |
| `kind` | `'read' \| 'write' \| 'window'` | Classification of whether the operation reads state, writes state or interacts with a window. |
| `params` | `object` | Draft-07 JSON Schema describing accepted parameters, including routing fields when applicable. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/app/operations \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const ops = await curio.app.operations();
    const names = ops.map((op) => op.name);
    ```

=== "MCP"

    ```json
    {
      "name": "app_operations",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  {
    "name": "app.info",
    "description": "Curio's version, the API version, the display language, and the system it runs on: `platform` as `windows`, `linux` or `macos`, `arch` as `x86_64` or `aarch64`",
    "permission": "library.read",
    "kind": "read",
    "params": {
      "type": "object",
      "properties": {}
    }
  },
  {
    "name": "item.count",
    "description": "Count the items a query finds",
    "permission": "metadata.read",
    "kind": "read",
    "params": {
      "type": "object",
      "properties": {
        "query": { "type": "string" },
        "scope": { "type": "string", "description": "all (default), uncategorized, untagged, recent, trash or missing." },
        "library": { "type": "string", "description": "The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window." }
      }
    }
  }
]
```

**Side effects, events and errors**

- Emits no public events and modifies no state.
- Errors: `unauthorized`, `not_allowed` (missing `library.read`), `bad_request` (if `library` or `window` is passed).
- Related: [`app.info`](#app-info), [Tokens and permissions](../connect/authorization.md).

---

## app.fonts {#app-fonts}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/app/fonts</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: app_fonts</span>
</div>

Lists the font family names installed on the local computer.

**Parameters**

This operation takes no parameters (`{}`).

**Returns**

`string[]` — array of installed system font family names.

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/app/fonts \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const families = await curio.app.fonts();
    ```

=== "MCP"

    ```json
    {
      "name": "app_fonts",
      "arguments": {}
    }
    ```

**Example response**

```json
[
  "Arial",
  "Cascadia Mono",
  "Georgia",
  "Inter",
  "Segoe UI"
]
```

**Side effects, events and errors**

- Emits no public events and modifies no state.
- Errors: `unauthorized`, `not_allowed` (missing `library.read`), `bad_request` (if `library` or `window` is passed).
- Related: [`view.setSample`](./view.md#view-setsample).

---

## app.theme {#app-theme}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/app/theme</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: app_theme</span>
</div>

Returns the active theme shown by a Curio window: its name, whether it is dark and its complete map of CSS design tokens (colors, corner radii, spacing scales and fonts). Unlike library operations, this never wakes a background process; a real Curio window must be open. Plugins always read the theme of their own host window and cannot pass `window` or `library`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `window` | `string` | No | Most recently focused window | Routing parameter: window ID from [`library.list`](./libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Most recently focused window | Routing parameter: library ID, folder path or name whose open window should answer. Not allowed for plugin callers. |

See [Choose a library or window](../connect/routing.md) for window routing precedence.

**Returns**

[`Theme`](../types/library.md#theme):

| Field | Type | Description |
|---|---|---|
| `name` | `string` | Active theme name. |
| `dark` | `boolean` | `true` when the active theme is dark; `false` when light. |
| `tokens` | `Record<string, string>` | Map of CSS custom property names (without leading `--`) to their resolved values, including colors (`bg`, `panel`, `surface`, `text`, `accent`), radii (`r`, `round`), spacing (`g`, `row`) and fonts (`ui-font`, `mono-font`). |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/app/theme \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const theme = await curio.app.theme();
    document.documentElement.dataset.dark = String(theme.dark);
    ```

=== "MCP"

    ```json
    {
      "name": "app_theme",
      "arguments": {}
    }
    ```

**Example response**

```json
{
  "name": "Dark",
  "dark": true,
  "tokens": {
    "bg": "#181a1b",
    "panel": "#1f2224",
    "surface": "#262a2d",
    "text": "#eceef0",
    "accent": "#3b9160",
    "r": "6px",
    "round": "999px",
    "g": "8px",
    "row": "30px",
    "ui-font": "Inter, system-ui, sans-serif",
    "mono-font": "Cascadia Mono, monospace"
  }
}
```

**Side effects, events and errors**

- Emits no public events on read; subscribe to [`app.themeChanged`](../events/libraries.md#app-themechanged) to track theme switches live.
- Errors: `no_window` (no matching Curio window is open), `busy` (the window has not finished reporting its initial theme yet), `not_found` (unknown `window` ID), `bad_request` (`window` and `library` disagree), `not_allowed` (missing `library.read`, unreachable library or plugin attempting to target another window).
- Related: [`app.themeChanged`](../events/libraries.md#app-themechanged), [Make pages look like Curio](../../plugins/pages/appearance.md).

---

## app.openUrl {#app-openurl}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/app/openUrl</span>
  <span class="curio-badge">Permission: ui</span>
  <span class="curio-badge curio-badge--muted">Target: Window</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: app_openUrl</span>
</div>

Prompts the user in a Curio window with a confirmation dialog displaying the full URL and opens that URL in the system's default browser or mail client only if the user clicks **Open**. Curio waits up to 10 minutes (600 seconds) for the user to respond. Only `http://`, `https://` and `mailto:` schemes are accepted.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `url` | `string` | Yes | — | Target address to open. Leading/trailing whitespace is trimmed; maximum 2,000 characters; must start with `http://`, `https://` or `mailto:` and contain no control characters. |
| `window` | `string` | No | Most recently focused window | Routing parameter: window ID from [`library.list`](./libraries.md#library-list). Not allowed for plugin callers. |
| `library` | `string` | No | Most recently focused window | Routing parameter: library ID, folder path or name whose open window displays the confirmation dialog. Not allowed for plugin callers. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `opened` | `boolean` | `true` if the user confirmed and the URL was handed to the OS handler; `false` if the user canceled or dismissed the prompt. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/app/openUrl \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"url": "https://example.com/documentation"}'
    ```

=== "JavaScript"

    ```js
    const { opened } = await curio.app.openUrl({ url: 'https://example.com/documentation' });
    ```

=== "MCP"

    ```json
    {
      "name": "app_openUrl",
      "arguments": {
        "url": "https://example.com/documentation"
      }
    }
    ```

**Example response**

```json
{
  "opened": true
}
```

**Side effects, events and errors**

- Brings the targeted window forward to display the confirmation dialog; launches the system handler only on explicit confirmation.
- Emits no public events.
- Errors: `bad_request` (URL exceeds 2,000 characters, contains control characters or uses a scheme other than `http://`, `https://` or `mailto:`), `no_window` (no matching window is open), `busy` (the window is already showing another modal prompt or the 600-second prompt wait timed out), `failed` (the OS failed to launch the browser/mail client), `not_allowed` (missing `ui` permission).
- Related: [`item.open`](./item-open.md#item-open), [`ui.confirm`](./ui.md#ui-confirm).
