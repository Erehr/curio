# Client

`curio.js` is Curio's zero-dependency JavaScript client. It works across browsers, Node.js (18+), Tampermonkey and Violentmonkey userscripts, browser extensions, plugin pages and plugin Node.js programs.

The running application serves both the client and its TypeScript declarations:

- `http://127.0.0.1:41733/api/v1/curio.js` (and `/curio/ui/curio.js` inside plugin pages)
- `http://127.0.0.1:41733/api/v1/curio.d.ts`

## Curio.connect(options)

Create a client bound to a transport, token and optional default library:

```javascript
const curio = Curio.connect({
  token: 'curio_...',
  host: '127.0.0.1',
  port: 41733,
  library: 'Design',
})
```

| Option | Type | Default | Description |
|---|---|---|---|
| `token` | `string` | `process.env.CURIO_API_TOKEN` (plugin Node programs) or omitted in plugin pages | Bearer token from **Settings › API**. Required for HTTP callers outside plugin pages and plugin programs. |
| `url` | `string` | `http://<host>:<port>/api/v1/` | Full base API URL; overrides `host` and `port`. Populated from `CURIO_API_URL` in plugin Node programs. |
| `host` | `string` | `'127.0.0.1'` | Loopback host name. |
| `port` | `number` | `41733` | Port configured in **Settings › API**. |
| `library` | `string` | None (uses default library) | Library ID, folder path or name sent as `?library=` on every call. |
| `transport` | `'fetch' \| 'gm' \| 'bridge'` | Auto-detected | Forces a specific transport instead of automatic detection. |

Calling `Curio.connect()` without arguments works inside **plugin pages** (using `bridge`) and **plugin Node.js programs** (reading `CURIO_API_TOKEN` and `CURIO_API_URL` from `process.env`).

## Curio.apiVersion

`Curio.apiVersion` is the integer API version supported by the client (currently `1`). Compare it against `apiVersion` from [`app.info`](../operations/app.md#app-info) to verify compatibility.

## client.options

`client.options` returns a shallow copy of the options passed to `Curio.connect()` with `token` set to `undefined` so logging never leaks credentials.

- On HTTP callers, `client.options` keeps whatever properties you passed (plus `url` from `CURIO_API_URL` in plugin Node.js programs when `token` was omitted). Unspecified defaults such as `host`, `port` or `transport` are not filled in.
- On plugin pages, `client.options` includes `transport: 'bridge'` alongside any options passed to `Curio.connect()`.

## `client.<namespace>.<method>(params)`

The client wraps `call()` in a recursive JavaScript `Proxy` so every dotted operation can be called as a nested method:

```javascript
const info = await curio.app.info()
const page = await curio.item.query({ query: 'tag = sky AND rating >= 4', limit: 20 })
const added = await curio.item.add.fromUrls({ urls: ['https://example.com/a.png'] })
const groups = await curio.tagGroup.list()
```

JSON operations resolve to parsed JavaScript values. Binary operations ([`item.file`](../operations/item-files.md#item-file), [`item.thumbnail`](../operations/item-files.md#item-thumbnail) and [`archive.entry`](../operations/archive.md#archive-entry)) resolve to a `Blob` with the response `Content-Type`.

## client.call(op, params)

Invoke any operation dynamically by its canonical dotted name:

```javascript
const res = await curio.call('item.count', { query: 'ext = png' })
```

## client.forLibrary(library)

Returns a derived client pinned to a specific library (ID, name or folder path) while reusing the same token and transport:

```javascript
const photos = curio.forLibrary('Photos')
const refs = curio.forLibrary('References')
const [pCount, rCount] = await Promise.all([
  photos.item.count({}),
  refs.item.count({}),
])
```

Inside a plugin page, calling `forLibrary()` throws `Curio.Error` (`code: 'not_allowed'`) because plugins operate only on their launching window's library.

## client.subscribe(filter, onEvent, onState)

Opens a real-time event subscription over `POST /api/v1/events` for HTTP callers or the window bridge inside plugin pages:

```javascript
const sub = curio.subscribe(
  { events: ['items.*', 'folders.changed'], library: 'Design' },
  (event) => console.log(event.name, event.data),
  (state, err) => console.log('stream state:', state, err?.message || ''),
)
```

| Parameter | Type | Description |
|---|---|---|
| `filter` | `EventFilter` | `{ events?: string[], library?: string, ids?: string[], after?: string }` |
| `onEvent` | `(event: Event) => void` | Runs for each matching event `{ id, name, library, at, data }`. |
| `onState` | `(state: 'open' \| 'retrying' \| 'closed', err?: CurioError) => void` | Optional connection lifecycle callback. |

Over HTTP, `subscribe()` tracks the last received `event.id`, reconnects every **3 seconds** after network drops or router handovers and replays missed events with `after`. HTTP `4xx` refusals (`400`, `401`, `403`, `404`, `409`) stop reconnecting and report `'closed'` with the error.

## client.on(name, onEvent, filter)

Subscribes to a single event name or namespace wildcard (such as `'items.*'` or `'plugin.*'`):

```javascript
const sub = curio.on('items.added', (e) => {
  console.log('Added:', e.data.ids)
})
```

## Subscription.close()

Both `subscribe()` and `on()` return a `Subscription` with a `close()` method. Calling `close()` aborts the SSE request (or unregisters the plugin bridge listener) and sets state to `'closed'`:

```javascript
sub.close()
```

## Curio.Error

Every failed call rejects with an instance of `Curio.Error` (`name === 'CurioError'`):

| Property | Type | Description |
|---|---|---|
| `code` | `string` | Machine-readable error code: `bad_request`, `unauthorized`, `not_allowed`, `not_found`, `unknown_operation`, `unknown_library`, `library_closed`, `library_offline`, `no_window`, `too_large`, `busy`, `failed` or `network`. |
| `status` | `number` | HTTP status code (`0` for network failures or plugin bridge calls). |
| `message` | `string` | Human-readable explanation. |

```javascript
try {
  await curio.item.get({ id: 'missing-id' })
} catch (err) {
  if (err instanceof Curio.Error && err.code === 'not_found') {
    console.warn('Item does not exist')
  } else {
    throw err
  }
}
```

## Environments

`curio.js` picks the right transport automatically:

1. **Plugin page bridge (`bridge`)**: Selected inside a framed `plugin://` or `http://plugin.<id>.localhost/` page without an explicit token. It uses `window.parent.postMessage`, forwards unhandled keys and pointer positions and applies live theme tokens (`--bg`, `--accent`). It also dispatches `curio-theme` and `curio-language` events and tracks widget height and `document.title`.
2. **Userscript GM request (`gm`)**: Selected when `GM_xmlhttpRequest` or `GM.xmlHttpRequest` is available, bypassing page CSP and CORS to reach `127.0.0.1`. Tampermonkey streams SSE through GM, while Violentmonkey falls back to `fetch` because it buffers responses until completion.
3. **Standard Fetch (`fetch`)**: Used in browsers, browser extensions and Node.js 18+. In CommonJS Node.js scripts, load `curio.js` with `const Curio = require('./curio.js')`; in browser scripts, loading `curio.js` sets `globalThis.Curio`.

## Types & helpers

Download `/api/v1/curio.d.ts` into your TypeScript or JSDoc project for autocomplete across all **164** registered operations, parameter interfaces, return types and `EventMap`.

Inside **hidden plugin worker pages**, the bridge client exposes two page-only completion helpers:

- **`client.thumbnailDone(request, picture, info)`**: Answers a [`plugin.thumbnail`](../../plugins/hooks/work.md#plugin-thumbnail) request with a `Blob`, `data:` URL, base64 string or `null` plus optional `{ width, height }`.
- **`client.extractDone(request, values, error)`**: Answers a [`plugin.extract`](../../plugins/hooks/work.md#plugin-extract) request with a `{ [fieldKey]: value }` map or an error string.

See [Complete a thumbnail or extraction request](../../plugins/pages/completions.md) for details.
