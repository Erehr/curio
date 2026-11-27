# Client

TypeScript declarations shipped in `GET /api/v1/curio.d.ts` under `declare namespace Curio`, plus runtime extensions supported by `curio.js`.

## Options {#options}

Configuration object accepted by [`Curio.connect(options)`](../connect/client.md#curioconnectoptions):

```typescript
interface Options {
  token: string;
  port?: number;
  host?: string;
  library?: string;
  transport?: 'fetch' | 'gm';
}
```

Runtime additions in `curio.js` (not declared in `curio.d.ts`):

- **Optional `token`**: Omit `token` (or call `Curio.connect()` with no arguments) inside plugin pages (uses the `bridge` transport) or plugin Node.js programs (reads `CURIO_API_TOKEN`).
- **`url?: string`**: Full base API URL overriding `host` and `port` (read from `CURIO_API_URL` in plugin Node.js programs).
- **`transport: 'bridge'`**: Forces the sandboxed postMessage bridge inside plugin pages.

## Client {#client}

Return type of `Curio.connect(options)` combining [`Api`](#api) namespaces with core client methods:

```typescript
interface Client extends Api {
  call(op: string, params?: Record<string, unknown>): Promise<any>;
  subscribe(filter: EventFilter, onEvent: (e: Event) => void, onState?: (state: 'open' | 'retrying' | 'closed', error?: CurioError) => void): Subscription;
  on<N extends keyof EventMap>(name: N, onEvent: (e: Event<N>) => void, filter?: Omit<EventFilter, 'events'>): Subscription;
  forLibrary(library: string): Client;
}
```

Runtime additions in `curio.js` (not declared in `curio.d.ts`):

- **`client.options`**: Shallow copy of active connection options with `token` set to `undefined` on HTTP clients.
- **Wildcard and plugin names in `on()`**: Accepts namespace wildcards (`'items.*'`) and plugin page hooks (`'plugin.*'`) at runtime in addition to `keyof EventMap`.
- **Worker completion helpers**: Plugin bridge clients also expose `thumbnailDone(request, picture, info?)` and `extractDone(request, values, error?)` for hidden thumbnail and extractor pages.
- **`Curio.apiVersion`**: Top-level constant (`1`) exported alongside `Curio.connect` and `Curio.Error`.

## Api {#api}

Typed namespace proxy tree mapping every canonical operation to a method returning a `Promise`:

- **App and libraries**: `client.app.*`, `client.token.*`, `client.library.*`, `client.library.grouping.*`.
- **Items**: `client.item.*`, `client.item.add.*`.
- **Organization and storage**: `client.folder.*`, `client.smartFolder.*`, `client.tag.*`, `client.tagGroup.*`, `client.field.*`, `client.location.*`.
- **Queries and utilities**: `client.query.*`, `client.archive.*`, `client.duplicates.*`.
- **Window and UI**: `client.selection.*`, `client.view.*`, `client.viewer.*`, `client.ui.*`, `client.dialog.*`, `client.clipboard.*`.
- **Plugins**: `client.plugin.state.*`, `client.plugin.itemState.*`, `client.plugin.settings.*`, `client.plugin.log`, `client.plugin.close`.
- **Jobs and processes**: `client.job.*`, `client.process.run`, `client.tool.list`.

## CurioError {#curioerror}

Error interface for failures rejected by `curio.js`, constructed via `Curio.Error`:

```typescript
interface CurioError extends Error {
  code: string;
  status: number;
}
```

At runtime, `Curio.Error` sets `name` to `'CurioError'` and defaults `status` to `0` for network failures and plugin bridge calls. See [Error reference](../connect/errors.md) for every `code` and HTTP `status`.

## EventFilter {#eventfilter}

Subscription filter passed to `client.subscribe()` and `POST /api/v1/events`:

```typescript
interface EventFilter {
  events?: (keyof EventMap | `${string}.*`)[];
  library?: string;
  ids?: string[];
}
```

At runtime, both `POST /api/v1/events` and `curio.js` HTTP `subscribe()` also accept `after?: string` to resume after a specific event ID (not declared in `curio.d.ts`). Plugin bridge subscriptions forward only `events` and `ids`.

## Event {#event}

Envelope delivered to `onEvent` callbacks and SSE `data:` frames:

```typescript
interface Event<N extends keyof EventMap = keyof EventMap> {
  id: string;
  name: N;
  library: string | null;
  at: number;
  data: EventMap[N];
}
```

Page-local plugin hooks (such as `plugin.beforeExit`) deliver `id: null` and `library: null` at runtime.

## EventMap {#eventmap}

Maps all 24 public event names to their exact `data` payload types:

- **Application and library**: [`app.themeChanged`](../events/libraries.md#app-themechanged), [`library.listChanged`](../events/libraries.md#library-listchanged), [`library.opened`](../events/libraries.md#library-opened), [`library.closed`](../events/libraries.md#library-closed), [`library.switched`](../events/libraries.md#library-switched).
- **Items and organization**: [`items.added`](../events/items.md#items-added), [`items.changed`](../events/items.md#items-changed), [`items.removed`](../events/items.md#items-removed), [`folders.changed`](../events/items.md#folders-changed), [`smartFolders.changed`](../events/items.md#smartfolders-changed), [`tags.changed`](../events/items.md#tags-changed), [`tagGroups.changed`](../events/items.md#taggroups-changed), [`fields.changed`](../events/items.md#fields-changed).
- **Locations and scans**: [`locations.changed`](../events/scans.md#locations-changed), [`scan.progress`](../events/scans.md#scan-progress), [`scan.done`](../events/scans.md#scan-done).
- **Window and viewer**: [`selection.changed`](../events/windows.md#selection-changed), [`view.changed`](../events/windows.md#view-changed), [`viewer.opened`](../events/windows.md#viewer-opened), [`viewer.closed`](../events/windows.md#viewer-closed).
- **Jobs**: [`job.started`](../events/jobs.md#job-started), [`job.progress`](../events/jobs.md#job-progress), [`job.output`](../events/jobs.md#job-output), [`job.finished`](../events/jobs.md#job-finished).

## Subscription {#subscription}

Handle returned by `client.subscribe()` and `client.on()`:

```typescript
interface Subscription {
  close(): void;
}
```

## Parameter types

Every operation's parameter interface is documented on its [Operations reference](../operations/app.md) page and exported in `GET /api/v1/curio.d.ts`.
