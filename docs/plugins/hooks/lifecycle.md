# Lifecycle

Page-local lifecycle hooks arrive in sandboxed plugin frames via `curio.on(...)` or `curio.subscribe({ events: ['plugin.*'] })`. Local hooks always arrive with `id: null` and `library: null` and require no extra permissions.

### Delivery

| Category | Hooks | Behavior |
|---|---|---|
| **Sticky** (last value replayed once to late subscribers) | `plugin.run`, `plugin.item`, `plugin.selection`, `plugin.keys` | Safe to register after async startup; the host replays the current value once on first subscribe. |
| **Queued** (buffered until a listener subscribes) | `plugin.thumbnail`, `plugin.extract` | Pending work requests wait in a host queue so nothing is lost while a hidden page boots. |
| **Live** | `plugin.show`, `plugin.hide`, `plugin.beforeExit`, `plugin.settingsChanged`, `plugin.drop`, `plugin.dragOver`, `plugin.tool` | Delivered immediately when the event occurs. `plugin.beforeExit` also runs synchronously on `pagehide`. |

## plugin.run {#plugin-run}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Sticky</span>
  <span class="curio-badge curio-badge--muted">Roles: window, tab, popover</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event with the launch context when a window, tab or `ui.popover` page opens or is reactivated. Curio sends it once on first subscribe and again whenever a single-instance window or tab is re-triggered while open.

**Payload**

| Launch source | `data` shape |
|---|---|
| Window from Plugins menu or toolbar | `{ window: string, items: string[] }` |
| Tab from workspace bar | `{ tab: string }` |
| Action (`actions[].opens`) | `{ action: string, items: string[] }` |
| URL handler (`urls[].opens`) | `{ handler: string, urls: string[], place: { folders, tags, annotation, sourceUrl, rating, fields } }` |
| Popover (`ui.popover`) | Arbitrary `data` object passed by the caller |

```javascript
curio.on('plugin.run', ({ data }) => {
  if (data.urls) loadUrls(data.urls, data.place)
  else if (data.items) loadItems(data.items)
})
```

---

## plugin.show {#plugin-show}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: all visible frames</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event when the host window is restored or the user switches back to a tab plugin's page. Payload is `{}`.

```javascript
curio.on('plugin.show', () => resumeRenderLoop())
```

---

## plugin.hide {#plugin-hide}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: all visible frames</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event when the host window is minimized or hidden, or when the user leaves a tab plugin's page (the iframe stays loaded in memory). Payload is `{}`. Pause animations, WebGL loops or polling timers here.

```javascript
curio.on('plugin.hide', () => pauseRenderLoop())
```

---

## plugin.beforeExit {#plugin-beforeexit}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live (synchronous on pagehide)</span>
  <span class="curio-badge curio-badge--muted">Roles: all frames</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event synchronously on `pagehide` as the frame unmounts or its window closes. Payload is `{}`. Perform only **synchronous** cleanup here (such as releasing object URLs or flushing `localStorage`); async API calls will not complete before teardown.

```javascript
curio.on('plugin.beforeExit', () => {
  for (const url of objectUrls) URL.revokeObjectURL(url)
})
```

---

## plugin.settingsChanged {#plugin-settingschanged}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Live</span>
  <span class="curio-badge curio-badge--muted">Roles: all sibling frames across windows</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event to every other open page of your plugin across all windows (for example from your **Settings › Plugins** page to your open viewer or widget) when [`plugin.settings.set`](../../api/operations/plugin.md#plugin-settings-set) saves updated preferences.

**Payload**: Full merged `Record<string, unknown>` settings object.

```javascript
curio.on('plugin.settingsChanged', ({ data }) => {
  applyTypography(data.fontSize, data.theme)
})
```

---

## plugin.keys {#plugin-keys}

<div class="curio-meta">
  <span class="curio-badge">Delivery: Sticky</span>
  <span class="curio-badge curio-badge--muted">Roles: all frames</span>
  <span class="curio-badge curio-badge--muted">Wildcard: plugin.*</span>
</div>

Curio sends this event with your plugin's active keyboard shortcuts from **Settings › Shortcuts** on first subscribe and whenever bindings change. Unassigned actions and keys are omitted.

**Payload**: `{ actions: Record<string, string>, viewer: Record<string, string> }`

```javascript
curio.on('plugin.keys', ({ data }) => {
  shortcutHint.textContent = data.viewer.bookmark || 'Unassigned'
})
```
