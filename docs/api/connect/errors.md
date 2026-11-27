# Errors

HTTP, `curio.js`, MCP and plugin bridge calls all share one set of machine-readable error codes.

## Envelope

When an HTTP API call fails, Curio returns a JSON body with `Content-Type: application/json`:

```json
{
  "error": {
    "code": "not_allowed",
    "message": "item.update needs the metadata.write permission, which Read only token doesn't have"
  }
}
```

| Error `code` | HTTP status | Meaning |
|---|---|---|
| `bad_request` | `400` | Malformed JSON, wrong `Content-Type`, unknown or invalid parameter, bad query syntax, invalid `library` or `window` value, or request body over the 64 MiB JSON cap. |
| `unauthorized` | `401` | Missing `Authorization: Bearer ...` header, unknown or revoked token, or expired plugin run token (`CURIO_API_TOKEN`). |
| `not_allowed` | `403` | Missing permission, disallowed `Origin` or `Host` header, library outside the token's allowed list, paired extension restriction or operation blocked for this caller type. |
| `not_found` | `404` | Item, folder, smart folder, tag, field, location, archive entry, job, file on disk or route (`/mcp` when disabled) does not exist. |
| `unknown_operation` | `404` | Operation name does not exist in Curio's registry. |
| `unknown_library` | `404` | No library matches `library`, or multiple libraries share the same name (pass the library UUID or path instead). |
| `library_closed` | `409` | Target library is closed and could not be opened for the request. |
| `library_offline` | `409` | Target library folder is unreachable on disk (such as an unplugged drive or missing path). |
| `no_window` | `409` | Window-targeted operation called when no matching Curio window is open (for example, only a background holder is running). |
| `too_large` | `413` | `POST /api/v1/upload` exceeded 2 GiB or left less than 512 MiB free on the data drive. |
| `busy` | `503` | Operation-level transient contention: over 16 open SSE streams, pairing already active, modal prompt open, window timeout or router handover. See [State & busy](#state-busy) for the plain-text HTTP request cap. |
| `failed` | `422` | Runtime I/O, SQLite, FFmpeg, download or operation failure. |
| `network` *(client-only)* | `0` | `curio.js` could not reach Curio at `127.0.0.1:<port>` (Curio is closed or the API is disabled). |

## Validation

- **Content-Type & empty bodies**: Non-empty `POST /api/v1/<ns>/<method>` requests must send `Content-Type: application/json` and a JSON object body. Empty or whitespace-only bodies default to `{}` and do not require `Content-Type`.
- **Unknown parameters**: Curio rejects unrecognized parameter keys on API operations and `POST /api/v1/events` filters with `400 bad_request`.
- **Query syntax**: Invalid expressions passed to `item.query`, `item.count`, `smartFolder.create` or `query.facets` return `400 bad_request` with the error position and reason.

## Auth errors

- **Host header**: Curio accepts only `127.0.0.1:<port>`, `localhost:<port>` or `[::1]:<port>` (case-insensitive) and returns `403 not_allowed` for any other `Host` value.
- **Origin & permissions**: See [CORS & origins](http.md#cors-origins) and [Authorization](authorization.md) for browser origin allowlists, paired extension limits and conditional permissions such as `files.outside`.

## Library errors

- **Ambiguous names**: When two libraries share a name, passing that name returns `404 unknown_library`. Pass the stable UUID from [`library.list`](../operations/libraries.md#library-list) instead.
- **Closed & offline libraries**: Closed libraries open automatically in the background (up to 30 seconds). Unreachable folders return `409 library_offline`; see [Routing](routing.md).

## State & busy

- **HTTP concurrency cap**: Exceeding 8 concurrent HTTP requests returns plain-text HTTP `503` (`Too many requests at once`) without a JSON error envelope. `curio.js` converts this response into `Curio.Error` with `code: 'busy'` and `status: 503`.
- **Operation-level `busy`**: Exceeding 16 open SSE streams, concurrent modal prompts or background startup timeouts return HTTP `503` with the standard JSON `busy` envelope. Retry `503` errors with backoff.
- **Missing window (`409 no_window`)**: Window operations fail when only a background holder is running. Call [`library.open`](../operations/libraries.md#library-open) first (requires `library.manage`) or wait for a visible window.

## Limits & retry

- **Safe import retries**: Pass `requestId` on `item.add.*` calls. Retrying within 10 minutes after a `network` or `503 busy` error returns the original result (`repeat: true`) without duplicates. See [Queue & wait](files.md#queue-wait).
- **Expired uploads**: Upload tickets expire after 10 minutes and work once. Re-upload bytes to `POST /api/v1/upload` if a ticket expires; see [Upload limits](files.md#upload-limits).

## By transport

| Caller | Error surface |
|---|---|
| **HTTP** | HTTP status code plus `{"error":{"code":"...","message":"..."}}` (plain text on the 8-request `503` limit) |
| **`curio.js`** | Rejects the Promise with `Curio.Error` (`err.code`, `err.status`, `err.message`) |
| **MCP (`tools/call`)** | Returns `{"jsonrpc":"2.0","id":...,"result":{"content":[{"type":"text","text":"..."}],"isError":true}}` |
| **Plugin page (`bridge`)** | Rejects the Promise with `Curio.Error` (`err.code`, `err.status === 0`, `err.message`) |
