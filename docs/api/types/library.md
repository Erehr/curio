# Libraries

Types returned by [`library.list`](../operations/libraries.md#library-list), [`library.info`](../operations/libraries.md#library-info), [`location.list`](../operations/location.md#location-list), [`app.theme`](../operations/app.md#app-theme) and related events.

## Library {#library}

An entry on this computer's library list (`library.list`, `library.listChanged`):

| Property | Type | Notes |
|---|---|---|
| `id` | `string \| null` | Stable library UUID across renames and moves (`null` until the library has been opened once). |
| `name` | `string` | Library name (its `.curio` folder stem). |
| `icon` | `string \| null` | Custom Tabler icon name, or `null` for default. |
| `color` | `string \| null` | Custom CSS color, or `null` for default. |
| `state` | `'window' \| 'background' \| 'closed' \| 'offline'` | `window`: open in a desktop window; `background`: held open by a headless process; `closed`: on disk, opened on demand; `offline`: folder unreachable. |
| `default` | `boolean` | `true` for the library targeted when a call specifies no `library`. |
| `window` | `string \| undefined` | Window label (e.g. `"main"`) when `state === 'window'`. Usable with window-targeted operations. |
| `path` | `string \| undefined` | Absolute `.curio` folder path. Included **only** when the caller holds `files.read`. |

```json
{
  "id": "c4b8f1d2-9a30-4e12-8b77-112233445566",
  "name": "Design",
  "icon": "palette",
  "color": "#3b9160",
  "state": "window",
  "default": true,
  "window": "main",
  "path": "C:\\Users\\You\\Libraries\\Design.curio"
}
```

## LibraryInfo {#libraryinfo}

Summary statistics for one open library ([`library.info`](../operations/libraries.md#library-info)):

| Property | Type | Notes |
|---|---|---|
| `id` | `string \| null` | Library UUID. |
| `name` | `string \| null` | Library name. |
| `schema` | `number` | Current database schema version. |
| `floor` | `number` | Minimum compatible schema floor required to open this library safely. |
| `counts` | `object` | `{ all, favorites, uncategorized, untagged, trash, missing, recent, tags, size }` where `size` is total item bytes. |
| `path` | `string \| undefined` | Absolute `.curio` folder path (requires `files.read`). |

```json
{
  "id": "c4b8f1d2-9a30-4e12-8b77-112233445566",
  "name": "Design",
  "schema": 1,
  "floor": 1,
  "counts": {
    "all": 4218,
    "favorites": 96,
    "uncategorized": 112,
    "untagged": 340,
    "trash": 7,
    "missing": 0,
    "recent": 48,
    "tags": 195,
    "size": 18492038144
  }
}
```

## Location {#location}

An indexed disk directory in a library ([`location.list`](../operations/location.md#location-list)):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric location ID within this library. |
| `name` | `string` | Display label shown in the sidebar. |
| `online` | `boolean` | `true` when the directory is reachable on disk. |
| `watch` | `boolean` | Whether OS filesystem watching is active for this location. |
| `access` | `'read_only' \| 'full' \| 'organize'` | Disk write policy (`read_only`: never write; `full`: edit files in place; `organize`: edit, rename and move). |
| `count` | `number` | Number of indexed items inside this location. |
| `lastScan` | `number \| null` | Timestamp of last completed scan (milliseconds since Unix epoch). |
| `icon` | `string \| null` | Custom Tabler icon name, or `null` (the UI picks a drive icon). |
| `color` | `string \| null` | Custom CSS color. |
| `path` | `string \| undefined` | Absolute folder path on disk (requires `files.read`). |

```json
{
  "id": 2,
  "name": "Stock Photos",
  "online": true,
  "watch": true,
  "access": "read_only",
  "count": 1520,
  "lastScan": 1763940100000,
  "icon": null,
  "color": "#4a90e2",
  "path": "D:\\Assets\\Stock Photos"
}
```

## Theme {#theme}

Active theme snapshot of a window ([`app.theme`](../operations/app.md#app-theme), [`app.themeChanged`](../events/libraries.md#app-themechanged)):

| Property | Type | Notes |
|---|---|---|
| `name` | `string` | Active theme name (e.g. `"Dark"`, `"Nord"`, `"Catppuccin"`). |
| `dark` | `boolean` | `true` for dark themes, `false` for light themes. |
| `tokens` | `Record<string, string>` | Map of CSS custom property names without leading `--` to resolved values. |

Token groups included in `tokens`:

- **Surfaces and text**: `bg`, `panel`, `surface`, `surface-2`, `surface-3`, `border`, `border-subtle`, `border-strong`, `text`, `text-muted` and `text-faint`.
- **Accent and status**: `accent`, `accent-text`, `accent-hover`, `accent-subtle`, `accent-border`, `selection`, `selection-text`, `info`, `danger`, `warning`, `success` and `star`, plus derived subtle and border status tokens.
- **Scale and typography**: `r`, `round`, `g`, `row`, `ui-font`, `mono-font`, `font-size` and any `--plugin-*` variables.

```json
{
  "name": "Dark",
  "dark": true,
  "tokens": {
    "bg": "#181a1b",
    "panel": "#1e2022",
    "surface": "#25282a",
    "text": "#e8eaed",
    "accent": "#3b9160",
    "ui-font": "Inter, system-ui, sans-serif",
    "mono-font": "JetBrains Mono, monospace"
  }
}
```
