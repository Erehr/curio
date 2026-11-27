# Types overview

Every response and event across HTTP, `curio.js`, MCP and plugins shares one JSON contract. Curio serves the TypeScript declarations at `GET /api/v1/curio.d.ts`.

## Identifiers

| Resource | Public identifier | Stability |
|---|---|---|
| **Item (`Item`)** | `id`: UUIDv7 string (RFC 9562, e.g. `"0194b7a2-..."`) | Permanent across moves, renames, relinks and exports. `ItemRef` parameters also accept an integer row ID or numeric string within the same library. |
| **Library (`Library`)** | `id`: UUID string (or `null` until opened once) | Stable across renames and folder moves on disk. |
| **Folder / Smart folder / Tag / Tag group / Field / Location** | `id`: integer (`number`) | Unique within that library. `TagRef` parameters accept either a tag name (`string`) or a numeric `id`. |
| **Job (`Job`)** | `id`: opaque string | Valid while running and for 10 minutes after completion. |
| **Window** | `window`: label string (`"main"`, `"main-2"`, etc.) | Valid while that window stays open. |

## Conventions

| Convention | Format |
|---|---|
| **Key casing** | All public API JSON keys use `camelCase`. |
| **Timestamps** | Unix epoch milliseconds (`number`, e.g. `1763941200000`), or `null` when unknown. |
| **Sizes** | Bytes (`number`). |
| **Durations** | Floating-point seconds (`number`) or `null`. |
| **Dimensions** | Pixels (`number \| null`), populated for images and video. |
| **Colors** | CSS hex (`#rrggbb`) or theme token (`var(--accent)`). |
| **Icons** | Tabler icon names without prefix (e.g. `"palette"`, `"folder"`, `"book"`). |

## Envelopes

- **Paginated lists**: `item.query`, `folder.items` and `smartFolder.items` return [`Page<T>`](item.md#page) (`{ total, offset, items }`). Default `limit` is **50**; hard cap is **1,000**.
- **Acknowledgements**: Mutations with no extra payload return [`Done`](runtime.md#done) (`{ "success": true }`).
- **One-off shapes**: Inline return shapes (such as `{ count: number }` or `{ tags: Record<string, string> }`) appear on their operation pages.

## Permissions

Curio omits filesystem paths unless the caller holds the required permission:

- **Library and item paths**: `Library.path`, `LibraryInfo.path`, `Location.path` and `Item.path` require `files.read`.
- **Tool paths**: `Tool.path` requires `exec`.

## Events {#events-params}

Every SSE and plugin-bridge event shares the envelope `{ id, name, library, at, data }`. See [Client and event types](client.md#event) and [Subscribe to the event stream](../connect/events.md).

## Type index

| Type group | Types documented | Page |
|---|---|---|
| **Items and pages** | `Kind`, `Item`, `ItemMember`, `ItemWithFields`, `Page<T>`, `Added` | [Items and pages](item.md) |
| **Libraries, storage and theme** | `Library`, `LibraryInfo`, `Location`, `Theme` | [Libraries, storage and theme](library.md) |
| **Folders, tags and metadata** | `Folder`, `SmartFolder`, `Tag`, `TagGroup`, `Field`, `FieldCoverage`, `FieldFacets`, `Facets` | [Folders, tags and metadata](organizing.md) |
| **Views, operations and jobs** | `View`, `Operation`, `Job`, `ProcessRun`, `Tool`, `Done` | [Views, operations and jobs](runtime.md) |
| **Client and event types** | `Options`, `Client`, `Api`, `CurioError`, `EventFilter`, `Event`, `EventMap`, `Subscription` | [Client and event types](client.md) |
