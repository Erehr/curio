# Events

Curio publishes real-time changes over **Server-Sent Events (SSE)** at `POST /api/v1/events`. One stream on the router port (`127.0.0.1:41733`) aggregates events from every open library window and background library holder on your computer.

!!! note "Open streams keep background library holders alive"
    Leaving an event stream open prevents idle background library holders from shutting down. Browser extensions and background scripts should call `sub.close()` (or abort the HTTP request) when inactive.

## Endpoint

Open a stream by sending a `POST` request with a JSON filter body and your bearer token:

```bash
curl -N -X POST http://127.0.0.1:41733/api/v1/events \
  -H "Authorization: Bearer curio_your_token_here" \
  -H "Content-Type: application/json" \
  -d '{"events":["items.*","folders.changed"]}'
```

Curio responds with `200 OK` and `Content-Type: text/event-stream`. Each event arrives as a standard SSE frame:

```text
id: 1763941200421-a3f9c012-42
event: items.added
data: {"id":"1763941200421-a3f9c012-42","name":"items.added","library":"c4b8f1d2-...","at":1763941200421,"data":{"ids":["91c0..."],"how":"api","requestId":"req-7"}}
```

## Envelope

Every SSE `data:` line is a JSON object with five fields:

| Field | Type | Description |
|---|---|---|
| `id` | `string` | Unique event ID formatted as `<at>-<source>-<seq>` (monotonic within each Curio process). |
| `name` | `string` | Canonical event name (such as `items.added`, `selection.changed` or `job.progress`). |
| `library` | `string \| undefined` | Stable UUID of the library that emitted the event; omitted for app-wide events such as `library.listChanged`. |
| `at` | `number` | Unix timestamp in milliseconds when Curio published the event. |
| `data` | `object` | Event-specific payload; see [Public events](../events/libraries.md). |

## Filters

The JSON filter body accepts four optional keys. Curio rejects unknown keys with `400 bad_request`.

| Filter key | Type | Description |
|---|---|---|
| `events` | `string[]` | Exact event names (`"items.added"`) or namespace wildcards (`"items.*"`, `"job.*"`). Omitting it subscribes to every event your token can read. |
| `library` | `string` | Restricts delivery to one library (by ID, name or folder path). App-wide `library.listChanged` events still arrive. |
| `ids` | `string[]` | Restricts `items.*` events to specific item UUIDs; requires `library`. |
| `after` | `string` | Resumes after this event ID (same effect as the HTTP `Last-Event-ID` header). |

## Permissions

Curio omits any event your caller lacks permission to read:

- **Per-event grants**: Each event requires `library.read`, `metadata.read` or `ui`. Events outside your token's permissions are omitted.
- **Library restrictions**: Tokens scoped to specific **Allowed libraries** receive events only from those libraries. `library.listChanged` still arrives with the library list filtered to what that token can see.
- **Plugin pages**: Inside plugin pages, `curio.subscribe()` scopes remote events to the plugin's originating library and merges local page hooks (`plugin.*`) into the same callback.

## Heartbeats & resume

- **Heartbeats**: Curio writes `: connected` and `retry: 3000` when a stream opens, then sends `: ping` every **15 seconds** of inactivity to keep connections alive.
- **Resuming**: Pass the last processed `event.id` in the HTTP `Last-Event-ID` header or the JSON body's `after` field to replay missed events after a disconnect.
- **Automatic reconnect**: `curio.js`'s `subscribe()` tracks the latest `event.id` and reconnects after **3 seconds** on transient errors.

## Replay & lag

Each Curio process keeps recent event history in bounded memory:

| Behavior | Limit or rule |
|---|---|
| Replay window | Retains up to **2,000 events** or **120 seconds**, whichever limit hits first. |
| Router handover | If the router process closes and another window takes over port `41733`, the new router replays all retained events with `at >= timestamp(after)` (excluding `after` itself). Deduplicate by `event.id` to handle overlaps. |
| Lag protection | Each subscriber queue holds up to **4,096 events**. If a slow consumer falls further behind, Curio drops the connection so the client reconnects and resyncs. |
| Batching & coalescing | Large scans split `items.*` events into chunks of at most **1,000 IDs**. Thumbnail completions coalesce every **500 ms** into one `items.changed` (`what: "thumbnail"`). |

## Closing & resync

- **Close idle streams**: Abort the HTTP request or call `sub.close()` in `curio.js` when you no longer need updates.
- **Resync after gaps**: After an outage over 120 seconds or a lag disconnect, run a fresh read (`item.query`, `folder.list`, `tag.list`, `view.get`) before resuming updates.
