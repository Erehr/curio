# Runtime

Plugin pages are standard HTML, CSS and JavaScript documents rendered inside sandboxed iframes connected to Curio through `/curio/ui/curio.js`.

## Frame roles

Curio attaches a `role` to every mounted plugin frame and exposes it on `document.documentElement` as `.curio-role-<role>`:

| Role | Mounted where | Lifecycle |
|---|---|---|
| `window` | Dedicated standalone plugin window (`windows[]`) | Closed via titlebar or `plugin.close()`. |
| `tab` | Main workspace beside the grid and Tag Manager (`tabs[]`) | Stays loaded when switching tabs; receives `plugin.hide` and `plugin.show`. |
| `widget` | Inspector or Library sidebar (`widgets[].page`) | Resizes to body height (0 px hides the widget); receives `plugin.selection`; `document.title` updates the widget heading. |
| `viewer` | Main viewer pane (`formats[].viewer`) | Stays loaded across adjacent items; receives `plugin.item` and `plugin.tool`; drives the toolbar via `viewer.setTools`. |
| `settings` | Inline inside **Settings › Plugins** (`settings`) | Resizes to body height; reads and writes `plugin.settings`. |
| `popover` | Anchored overlay opened via `ui.popover` | Resizes to body height; receives `data` via `plugin.run`; closes with `plugin.close({ result })`. |
| `thumbnail` / `extractor` | Hidden background frames (`thumbnail_page`, `extractors[].page`) | Queued requests via `plugin.thumbnail` or `plugin.extract`; answered via `curio.thumbnailDone` or `curio.extractDone`. |

## Origins

Every plugin receives a dedicated origin computed from its ID and a hash (`<label>`):

- **Linux and macOS (WebKit)**: `plugin://<label>/<plugin-id>/<path>`
- **Windows (WebView2)**: `http://plugin.<label>.localhost/<plugin-id>/<path>`

Always reference your own scripts, CSS, WebAssembly and images with relative paths (`src="app.js"`, `href="style.css"`) and Curio's shared runtime with `/curio/ui/...` (`/curio/ui/curio.js`, `/curio/ui/curio.css`). Never hardcode the origin. Path traversal outside the plugin folder (including through symlinks) is refused, and no other plugin's files are served under your origin.

## Sandbox & CSP

Curio serves plugin pages with a strict Content Security Policy:

- **No inline `<script>` or `eval`**: Load JavaScript from external files (`<script src="/curio/ui/curio.js"></script>` and `<script type="module" src="app.js"></script>`).
- **No form navigation**: The sandbox omits `allow-forms`; handle button `click` events or call `e.preventDefault()` on `submit`.
- **WebAssembly & blobs**: WebAssembly (`wasm-unsafe-eval`) is allowed, along with `data:` or `blob:` images and `blob:` styles, fonts and child iframes (used by `foliate-js` book sections). Scripts inside `blob:` child iframes are blocked.
- **Network isolation**: Without `network`, `connect-src` allows only the plugin's own origin, `/curio/ui/`, `data:` and `blob:`. With `network`, it allows only the hostnames declared in `plugin.json`'s `network[]` (or any `https:` or `wss:` host when `network[]` is empty).
- **WebView storage**: Each plugin origin gets isolated `localStorage` and IndexedDB, but treat them as caches only and use [`plugin.state` or `plugin.settings`](state.md) for persistent data. Service workers are blocked.

## Connecting

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="/curio/ui/curio.css">
  <script src="/curio/ui/curio.js"></script>
  <script type="module" src="app.js"></script>
</head>
<body>
  <main id="root"></main>
</body>
</html>
```

```javascript
// app.js
const curio = Curio.connect()
const info = await curio.app.info()
```

Calling `Curio.connect()` with no arguments selects the `bridge` transport (`window.parent.postMessage`), authenticates as your plugin and needs no API token.

## Scoping

Every call from a plugin page targets the **originating library** (and its main window for `selection.*`, `view.*`, `viewer.*`, `ui.*` and `dialog.*`). Calling `curio.forLibrary(...)` throws `Curio.Error` (`code: "not_allowed"`).

## Focus & keys

`curio.js` forwards unhandled `keydown` events (outside `<input>`, `<textarea>`, `<select>` and `contenteditable`) to the Curio host window so navigation shortcuts (`ArrowLeft`, `ArrowRight`, `Escape` and viewer controls) continue working while a plugin page has focus:

- Call `event.preventDefault()` on any key your page handles itself to stop it from reaching Curio.
- Curio accepts forwarded keys only from the currently focused frame and ignores destructive shortcuts (rating, trash, paste, export and running actions) replayed from a plugin page.
- `curio.js` suppresses the WebView's default browser context menu (Back, Reload, Print) everywhere except inside text inputs or text selections.

## Lifecycle

- Subscribe to [`plugin.hide`](../hooks/lifecycle.md#plugin-hide) and [`plugin.show`](../hooks/lifecycle.md#plugin-show) to pause animations, WebGL loops or timers while your tab or window is hidden.
- Subscribe to [`plugin.beforeExit`](../hooks/lifecycle.md#plugin-beforeexit) (fired synchronously on `pagehide`) for final synchronous cleanup. Async API calls will not finish once `beforeExit` fires.
- Curio unsubscribes all remote event streams when an iframe detaches or reloads, and you should still call `sub.close()` on subscriptions you no longer need.
