# Libraries

Events published when a window's theme changes or when libraries open, close, switch or change on this computer's library list. All five events require `library.read`.

Subscribe over HTTP SSE by sending `{"events":["app.themeChanged"]}` or `{"events":["library.*"]}` to `POST /api/v1/events`, or use `curio.on(...)` below. See [Subscribe to the event stream](../connect/events.md) for stream filtering, heartbeats, replay windows and reconnection.

## app.themeChanged {#app-themechanged}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Window / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: app.*</span>
</div>

Emitted when a Curio window switches to a different theme or edits tokens of its active theme.

**Payload** (`Theme & { window: string }`)

| Field | Type | Description |
|---|---|---|
| `window` | `string` | Label of the window whose theme changed. |
| `name` | `string` | Active theme name (e.g. `"Dark"`, `"Nord"`). |
| `dark` | `boolean` | `true` for dark palettes, `false` for light. |
| `tokens` | `Record<string, string>` | Map of CSS token names (without `--`) to values. |

```javascript
const sub = curio.on('app.themeChanged', (ev) => {
  console.log(ev.data.window, ev.data.name, ev.data.tokens.accent)
})
```

**Observed event**

```json
{
  "id": "1763941200001-a1b2c3d4-7",
  "name": "app.themeChanged",
  "library": "c4b8f1d2-9a30-4e12-8b77-112233445566",
  "at": 1763941200001,
  "data": { "window": "main", "name": "Nord", "dark": true, "tokens": { "bg": "#2e3440", "accent": "#88c0d0" } }
}
```

Related: [`app.theme`](../operations/app.md#app-theme). Plugin pages also receive the DOM `curio-theme` event.

---

## library.listChanged {#library-listchanged}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: App-wide (library is null)</span>
  <span class="curio-badge curio-badge--muted">Wildcard: library.*</span>
</div>

Emitted when a library is created, opened, forgotten, renamed, moved, reordered or given a new icon or color. Even when a subscription filters to one `library`, this event still arrives with the list limited to libraries your token can access.

**Payload** (`{ libraries: Library[] }`)

| Field | Type | Description |
|---|---|---|
| `libraries` | [`Library[]`](../types/library.md#library) | Updated library list matching [`library.list`](../operations/libraries.md#library-list). |

```javascript
const sub = curio.on('library.listChanged', (ev) => {
  console.log('Known libraries:', ev.data.libraries.map((l) => l.name))
})
```

**Observed event**

```json
{
  "id": "1763941205120-a1b2c3d4-8",
  "name": "library.listChanged",
  "library": null,
  "at": 1763941205120,
  "data": { "libraries": [{ "id": "c4b8f1d2-...", "name": "Design", "state": "window", "default": true, "window": "main" }] }
}
```

---

## library.opened {#library-opened}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: library.*</span>
</div>

Emitted when a library opens in a desktop window or background holder, or when a background holder promotes to a visible window.

**Payload** (`{ name: string; state: 'window' | 'background' }`)

| Field | Type | Description |
|---|---|---|
| `name` | `string` | Library name. |
| `state` | `'window' \| 'background'` | Hosting mode. |

```javascript
curio.on('library.opened', (ev) => console.log(ev.library, ev.data.name, ev.data.state))
```

**Observed event**

```json
{
  "id": "1763941210300-a1b2c3d4-9",
  "name": "library.opened",
  "library": "c4b8f1d2-9a30-4e12-8b77-112233445566",
  "at": 1763941210300,
  "data": { "name": "Design", "state": "window" }
}
```

---

## library.closed {#library-closed}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: library.*</span>
</div>

Emitted when the last window or background holder for a library exits.

**Payload** (`{ name: string }`)

| Field | Type | Description |
|---|---|---|
| `name` | `string` | Library name. |

```javascript
curio.on('library.closed', (ev) => console.log('Closed:', ev.data.name))
```

**Observed event**

```json
{
  "id": "1763941220500-a1b2c3d4-10",
  "name": "library.closed",
  "library": "c4b8f1d2-9a30-4e12-8b77-112233445566",
  "at": 1763941220500,
  "data": { "name": "Design" }
}
```

---

## library.switched {#library-switched}

<div class="curio-meta">
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Scope: Window / Library</span>
  <span class="curio-badge curio-badge--muted">Wildcard: library.*</span>
</div>

Emitted when an open desktop window switches from one library to another.

**Payload** (`{ window: string; from: string; to: string }`)

| Field | Type | Description |
|---|---|---|
| `window` | `string` | Window label. |
| `from` | `string` | Previous library UUID. |
| `to` | `string` | Newly active library UUID. |

```javascript
curio.on('library.switched', (ev) => console.log(ev.data.window, ev.data.from, '->', ev.data.to))
```

**Observed event**

```json
{
  "id": "1763941230800-a1b2c3d4-11",
  "name": "library.switched",
  "library": "d9e1a0b2-...",
  "at": 1763941230800,
  "data": { "window": "main", "from": "c4b8f1d2-...", "to": "d9e1a0b2-..." }
}
```
