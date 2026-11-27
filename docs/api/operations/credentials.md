# token

Operations in the `token` namespace let an external web client revoke its own API token when disconnecting or signing out.

## token.forget {#token-forget}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/token/forget</span>
  <span class="curio-badge">Permission: library.read</span>
  <span class="curio-badge curio-badge--muted">Target: App</span>
  <span class="curio-badge curio-badge--muted">Callers: Web</span>
  <span class="curio-badge curio-badge--muted">MCP: none</span>
</div>

Permanently deletes the calling bearer token from Curio's saved API token list. Paired browser extensions and external tools call this when disconnecting so stale credentials do not remain in **Settings › API**.

Once the response is sent, the token stops working for HTTP requests, uploads and MCP calls. Any open SSE event streams using that token close within a few seconds.

Only Web API token callers may call this operation; MCP clients, plugins and internal window callers receive `not_allowed`. Passing `library` or `window` is rejected with `bad_request`.

**Parameters**

This operation takes no parameters (`{}`).

**Returns**

[`Done`](../types/runtime.md#done):

| Field | Type | Description |
|---|---|---|
| `success` | `true` | Always `true` once the token has been removed from configuration. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/token/forget \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    await curio.token.forget();
    // Subsequent calls with this client will fail with CurioError('unauthorized').
    ```

**Example response**

```json
{
  "success": true
}
```

**Side effects, events and errors**

- Deletes the calling token record from Curio's persistent configuration; the token cannot be recovered.
- Open `POST /api/v1/events` streams authenticated with the deleted token are terminated at the next grant check (every 3 seconds).
- Emits no public events.
- Errors: `unauthorized` (missing or already-revoked token), `not_allowed` (caller is not a Web API token caller, or token lacks `library.read`), `bad_request` (if `library` or `window` is passed, or caller has no token ID).
- Related: [Tokens and permissions](../connect/authorization.md), [Launch links and browser pairing](../connect/launch-pair.md).
