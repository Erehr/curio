# query

Operations for checking [Query language](../query/syntax.md) expressions and computing rating, shape, kind, extension and animation facet distributions. Both operations target a **library**.

## query.validate {#query-validate}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/query/validate</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: query_validate</span>
</div>

Compile a query string against the library's active field catalog and return either its match count or a structured parse/indexing error (always HTTP `200`, never failing with `400` on invalid syntax).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | No | `""` | Query expression to validate. |
| `scope` | `string` | No | `"all"` | `all`, `uncategorized`, `untagged`, `recent`, `trash` or `missing`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** `{ valid: true; count: number } | { valid: false; error: string }`.

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/query/validate \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"query":"rating >= 4 AND ext IN (png, webp)"}'
    ```

=== "JavaScript"

    ```javascript
    const check = await curio.query.validate({ query: 'rating >= 4 AND ext IN (png, webp)' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query_validate","arguments":{"query":"rating >= 4 AND ext IN (png, webp)"}}}
    ```

**Example response**

```json
{ "valid": true, "count": 84 }
```

When invalid:

```json
{ "valid": false, "error": "Field 'seed' isn't indexed (Settings › Fields)" }
```

**Events and errors** Read-only. Returns `400 bad_request` for an unknown `scope`, `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.

---

## query.facets {#query-facets}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/query/facets</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: query_facets</span>
</div>

Compute facet histograms (star ratings, shapes, kinds, extensions, animated counts and total) over items matching an optional query and scope. For distinct values of a single custom or reader field see [`field.facets`](field.md#field-facets); for item counts and byte totals by kind, extension, folder, tag or month see [`library.stats`](libraries.md#library-stats).

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `query` | `string` | No | `""` | Query string narrowing the facet population. |
| `scope` | `string` | No | `"all"` | `all`, `uncategorized`, `untagged`, `recent`, `trash` or `missing`. |
| `library` | `string` | No | Default library | Target library (ID, path or unique name). Not allowed for plugin callers. |

**Returns** [`Facets`](../types/organizing.md#facets).

**Example request**

=== "HTTP"

    ```bash
    curl -s -X POST http://127.0.0.1:41733/api/v1/query/facets \
      -H "Authorization: Bearer curio_..." \
      -H "Content-Type: application/json" \
      -d '{"query":"tag = concept"}'
    ```

=== "JavaScript"

    ```javascript
    const facets = await curio.query.facets({ query: 'tag = concept' })
    ```

=== "MCP"

    ```json
    {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_facets","arguments":{"query":"tag = concept"}}}
    ```

**Example response**

```json
{
  "rating": { "0": 12, "4": 18, "5": 9 },
  "shape": { "landscape": 24, "portrait": 11, "square": 4, "wide": 2, "tall": 0, "4:3": 3, "16:9": 15 },
  "kinds": { "image": 37, "video": 2 },
  "exts": { "png": 29, "jpg": 8, "mp4": 2 },
  "animatedExts": { "webp": 1 },
  "animated": 1,
  "total": 39
}
```

**Events and errors** Read-only. Returns `400 bad_request` on invalid query syntax or unknown `scope`, `403 not_allowed` without `metadata.read`, `404 unknown_library` or `409 library_closed` / `library_offline`.
