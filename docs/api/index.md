# API

Curio uses one operation registry across every integration surface. HTTP callers, `curio.js` scripts, MCP AI assistants, browser extensions and plugins call the same operations with the same parameter schemas, permissions and error codes.

The API calls library entries **items** (matching the `item.*` namespace); user-facing feature docs refer to the same records as **assets**.

```mermaid
flowchart LR
  HTTP["HTTP / curl / Python<br/>POST /api/v1/&lt;ns&gt;/&lt;method&gt;"] --> Router
  JS["curio.js (Browser / Node / Userscript / Extension)"] --> Router
  MCP["MCP Client (AI Assistants)<br/>POST /mcp"] --> Router
  PLUG["Plugin Pages & Programs<br/>Bridge / CURIO_API_TOKEN"] --> Router

  subgraph Router["Curio API Router (127.0.0.1:41733)"]
    REG["Unified Operation Registry<br/>164 operations · 11 permissions"]
    SSE["Event Hub & SSE Stream<br/>POST /api/v1/events"]
  end

  Router --> W1["Open Library Window(s)"]
  Router --> BG["Background Library Holder(s)"]
```

## Transports

| Caller | Transport | Authentication | Best for |
|---|---|---|---|
| **HTTP scripts & backends** | `POST http://127.0.0.1:41733/api/v1/<namespace>/<method>` | `Authorization: Bearer curio_...` token from **Settings › API** | Python, PowerShell, curl, Rust, shell pipelines and custom integrations. See [Web API and service discovery](connect/http.md). |
| **JavaScript / TypeScript** | Served client `/api/v1/curio.js` + `/api/v1/curio.d.ts` | Token (or automatic in plugin pages and plugin Node programs) | Userscripts (`GM_xmlhttpRequest`), browser extensions, Node.js tools and web apps. See [JavaScript and TypeScript client](connect/client.md). |
| **MCP clients (AI assistants)** | Streamable HTTP JSON-RPC at `POST http://127.0.0.1:41733/mcp` | Bearer token + tool allowlist in **Settings › MCP** | Claude Desktop, IDEs and agents querying or organizing libraries. See [Connect an MCP client](connect/mcp.md). |
| **Plugins (pages & programs)** | Sandboxed postMessage bridge (pages) or loopback HTTP (`CURIO_API_TOKEN`) | Plugin manifest permissions granted per library (implicit `library.read`, never `library.manage`) | Extending Curio's UI, viewers, metadata readers, actions and URL handlers. See [Plugins](../plugins/index.md). |

## Quick start

1. Open **Settings › API** in Curio and turn on **Enable API** (default port `41733`).
2. Create a token (for example with the **Read only** preset) and copy its secret (`curio_...`).
3. Call `app.info` or `item.query`:

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/item/query \
      -H "Authorization: Bearer curio_your_token_here" \
      -H "Content-Type: application/json" \
      -d '{"query":"rating >= 4","limit":5}'
    ```

=== "JavaScript"

    ```javascript
    // Load Curio from http://127.0.0.1:41733/api/v1/curio.js (or require/import it in Node)
    const curio = Curio.connect({ token: 'curio_your_token_here' })
    const page = await curio.item.query({ query: 'rating >= 4', limit: 5 })
    console.log(page.total, page.items)
    ```

=== "MCP"

    ```json
    {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "item_query",
        "arguments": {
          "query": "rating >= 4",
          "limit": 5
        }
      }
    }
    ```

## Operations

Every operation has a dotted name (`item.add.fromUrls`), an HTTP path (`POST /api/v1/item/add/fromUrls`), a `curio.js` method (`curio.item.add.fromUrls(...)`) and, where exposed to MCP, an underscored tool name (`item_add_fromUrls`).

| Task area | Namespace(s) | Reference page |
|---|---|---|
| App version, theme, fonts, caller capabilities, open URL | `app.*`, `token.*` | [Application](operations/app.md) · [Current token](operations/credentials.md) |
| List, open, create, rename, style, stats, switch libraries | `library.*` | [Libraries and statistics](operations/libraries.md) |
| Inspect, survey, apply or split file-grouping rules | `library.grouping.*` | [Library grouping rules](operations/library-grouping.md) |
| Search, count, fetch items, EXIF, PNG text, visual similarity | `item.*` | [Find and read items](operations/item-reads.md) |
| Import from paths, URLs, base64 bytes, bookmarks or beside an item | `item.add.*` | [Add files and links](operations/item-add.md) |
| Update title, notes, URL, rating, favorite, explicit, fields, tags, folders | `item.*` | [Edit items and membership](operations/item-metadata.md) |
| Download/stream files, thumbnails, previews, rotate, replace, write URLs | `item.*` | [Files, covers and derived previews](operations/item-files.md) |
| Manually group, split, detach, forget or re-allow grouping | `item.*` | [Group files into items](operations/item-grouping.md) |
| Duplicate, copy/move across libraries, export, trash, restore, purge | `item.*` | [Transfer, export and remove items](operations/item-lifecycle.md) |
| Open item in Curio/external app or reveal in file manager | `item.*` | [Open and reveal items](operations/item-open.md) |
| Manage folders, smart folders, tags, tag groups, fields, locations | `folder.*`, `smartFolder.*`, `tag.*`, `tagGroup.*`, `field.*`, `location.*` | [Folders](operations/folder.md) · [Smart folders](operations/smart-folder.md) · [Tags](operations/tag.md) · [Tag groups](operations/tag-group.md) · [Fields](operations/field.md) · [Locations](operations/location.md) |
| Validate queries, count facets, read/create archives, find/resolve duplicates | `query.*`, `archive.*`, `duplicates.*` | [Query](operations/query.md) · [Archive](operations/archive.md) · [Duplicates](operations/duplicates.md) |
| Control window selection, scope, search, filters, viewer, dialogs, clipboard | `selection.*`, `view.*`, `viewer.*`, `ui.*`, `dialog.*`, `clipboard.*` | [Selection](operations/selection.md) · [View](operations/view.md) · [Viewer](operations/viewer.md) · [UI](operations/ui.md) · [Dialogs](operations/dialog.md) · [Clipboard](operations/clipboard.md) |
| Plugin state/settings/logging, background jobs, external processes & tools | `plugin.*`, `job.*`, `process.*`, `tool.*` | [Plugin](operations/plugin.md) · [Jobs](operations/job.md) · [Process](operations/process.md) · [Tools](operations/tool.md) |

## Events & types

- **Query syntax & searchable fields**: Learn the expression grammar in [Query language](query/syntax.md) and look up built-in fields in [Searchable field dictionary](query/fields.md).
- **Resource shapes & TypeScript types**: Start at [Resource and type index](types/index.md), then see [Items and pages](types/item.md), [Libraries, storage and theme](types/library.md), [Folders, tags and metadata](types/organizing.md), [Views, operations and jobs](types/runtime.md) and [Client and event types](types/client.md).
- **Real-time event stream**: See how `POST /api/v1/events` filters, replays and reconnects in [Subscribe to the event stream](connect/events.md), then inspect all 24 public events across [Libraries](events/libraries.md), [Items & organization](events/items.md), [Locations & scans](events/scans.md), [Selection, view & viewer](events/windows.md) and [Jobs](events/jobs.md).

## Capabilities

Any authenticated caller can discover its permitted operations and JSON Schemas at runtime by calling [`app.operations`](operations/app.md#app-operations) (requires only `library.read`):

```javascript
const ops = await curio.app.operations()
console.log(ops.map((o) => `${o.name} (${o.permission})`))
```

For MCP clients, [`tools/list`](connect/mcp.md#toolslist) returns the exact subset of operations permitted by both the caller's token and the toggles enabled in **Settings › MCP**.

## Versioning and compatibility

- **Strict request parameters**: Every operation rejects unknown top-level parameters with `bad_request` so newer client options are never silently ignored. Feature-detect newer parameters with `app.operations`.
- **Additive responses and events**: Operation responses and SSE event payloads grow by adding fields; clients should ignore unrecognized fields. Removing a field or changing its meaning introduces a new operation or API version (`/api/v2/`).
