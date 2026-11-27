# MCP

Curio includes a built-in **Model Context Protocol (MCP)** server over loopback HTTP. AI assistants (Claude Code, Claude Desktop, IDE agents and custom LLM clients) can search, inspect, tag and organize your local libraries.

## Setup

MCP is **off by default** (`POST /mcp` returns `404` until enabled):

1. Open **Settings › API**, turn on **Enable API** and create a bearer token with minimal permissions (such as **Read only**).
2. Open **Settings › MCP** and turn on **Enable MCP**.
3. Turn off unused tool families or tools to save context tokens and limit access.

=== "Claude Code"

    ```bash
    claude mcp add --transport http curio http://127.0.0.1:41733/mcp \
      --header "Authorization: Bearer curio_your_token_here"
    ```

=== "JSON config (Claude Desktop / IDEs)"

    ```json
    {
      "mcpServers": {
        "curio": {
          "type": "http",
          "url": "http://127.0.0.1:41733/mcp",
          "headers": {
            "Authorization": "Bearer curio_your_token_here"
          }
        }
      }
    }
    ```

## Transport

| Property | Value |
|---|---|
| Endpoint | `POST http://127.0.0.1:41733/mcp` (loopback only, sharing the API port) |
| Protocol | Stateless Streamable HTTP with JSON-RPC 2.0 responses (MCP version `2025-03-26`) |
| Authentication | `Authorization: Bearer <token>` on every request |

## initialize

Returns server metadata, protocol version, tool capabilities and built-in agent instructions:

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": { "tools": { "listChanged": false } },
    "serverInfo": { "name": "Curio", "version": "1.0.0" },
    "instructions": "Local asset library. Items have a stable text `id`. Use item_query with the query language (e.g. `tag = sky AND rating >= 4`, `prompt CONTAINS \"blue hair\"`). library_list shows the libraries; most tools take an optional `library`."
  }
}
```

## ping

Checks server availability and returns an empty object `{}`:

```json
// Request
{ "jsonrpc": "2.0", "id": 2, "method": "ping" }

// Response
{ "jsonrpc": "2.0", "id": 2, "result": {} }
```

## tools/list

Returns `{ "tools": [...] }` where each entry includes `name`, `description` and `inputSchema` (JSON Schema Draft-07). Curio includes a tool only when all three checks pass:

1. The operation is eligible for MCP callers.
2. The bearer token holds the operation's required permission.
3. The tool is enabled in **Settings › MCP**.

## tools/call

Runs one tool by its underscored name (such as `item_query`, `item_add_fromUrls` or `tagGroup_addTags`):

```bash
curl -s http://127.0.0.1:41733/mcp \
  -H "Authorization: Bearer curio_your_token_here" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "item_query",
      "arguments": {
        "query": "rating >= 4 AND tag = portrait",
        "limit": 5
      }
    }
  }'
```

Library-scoped tools accept an optional `library` argument (ID, name or folder path). When omitted, Curio targets the most recently focused library window (or default library) and opens closed libraries in the background.

## JSON-RPC

Curio supports standard JSON-RPC 2.0 notifications, batch requests and protocol errors:

| Message type | Behavior |
|---|---|
| Notification (no `id`, such as `notifications/initialized`) | Returns HTTP `202 Accepted` with an empty body. |
| Batch (JSON array of messages) | Runs each entry and returns an array of responses, omitting notifications. |
| Protocol errors | Invalid JSON returns `-32700` (parse error); unknown methods return `-32601` (`method not found`). |

## Results & limits

| Response type | Shape & limits |
|---|---|
| Standard JSON result | `{ "content": [{ "type": "text", "text": "<pretty JSON>" }], "isError": false }` |
| Thumbnail image (`item_thumbnail`) | `{ "content": [{ "type": "image", "data": "<base64>", "mimeType": "image/webp" }], "isError": false }` so vision models can inspect thumbnails directly. Images over **4 MiB** return an error instead. |
| Tool error | Unknown tools, tools disabled in **Settings › MCP**, permission denials, validation errors and runtime failures return `{ "content": [{ "type": "text", "text": "<error message>" }], "isError": true }`. |

## Tool index

MCP tool names replace every `.` in the canonical operation name with `_`, exposing up to **140** tools when every permission and toggle is enabled. Curio excludes 24 operations from MCP:

| Category | Excluded operations |
|---|---|
| Plugin page URL & UI helpers | `item.fileUrl`, `item.previewUrl`, `item.writeUrl`, `archive.entryUrl`, `viewer.setTools`, `ui.choose`, `ui.popover`, `plugin.close` |
| Plugin-only state & execution | `plugin.state.*`, `plugin.itemState.*`, `plugin.settings.*`, `plugin.log`, `process.run` |
| Internal job lifecycle | `job.start`, `job.update`, `job.finish`, `job.cancel` |
| Token & raw file downloads | `token.forget`, `item.file`, `archive.entry` |

| Namespace | Exposed MCP tools | Reference |
|---|---|---|
| `app` | `app_info`, `app_operations`, `app_fonts`, `app_theme`, `app_openUrl` | [Application](../operations/app.md) |
| `library` | `library_list`, `library_info`, `library_open`, `library_create`, `library_rename`, `library_forget`, `library_place`, `library_setLook`, `library_stats`, `library_switch` | [Libraries](../operations/libraries.md) |
| `library.grouping` | `library_grouping`, `library_grouping_survey`, `library_grouping_apply`, `library_grouping_split` | [Library grouping](../operations/library-grouping.md) |
| `item` (reads) | `item_query`, `item_count`, `item_get`, `item_getMany`, `item_readFields`, `item_exif`, `item_pngText`, `item_similar` | [Item reads](../operations/item-reads.md) |
| `item.add` | `item_add_fromPaths`, `item_add_fromUrls`, `item_add_fromBytes`, `item_add_bookmark`, `item_add_beside` | [Item add](../operations/item-add.md) |
| `item` (metadata) | `item_update`, `item_rename`, `item_addTags`, `item_removeTags`, `item_addToFolders`, `item_removeFromFolders`, `item_moveToFolder` | [Item metadata](../operations/item-metadata.md) |
| `item` (files) | `item_thumbnail`, `item_rotate`, `item_rotateFile`, `item_replaceFile`, `item_setThumbnail`, `item_refreshThumbnail`, `item_refreshPalette` | [Item files](../operations/item-files.md) |
| `item` (grouping) | `item_group`, `item_split`, `item_detach`, `item_forgetFile`, `item_allowGrouping` | [Item grouping](../operations/item-grouping.md) |
| `item` (lifecycle & open) | `item_duplicate`, `item_copyToLibrary`, `item_moveToLibrary`, `item_export`, `item_trash`, `item_restore`, `item_purge`, `item_open`, `item_reveal` | [Item lifecycle](../operations/item-lifecycle.md) · [Item open](../operations/item-open.md) |
| `folder` | `folder_list`, `folder_get`, `folder_recent`, `folder_items`, `folder_create`, `folder_rename`, `folder_move`, `folder_update`, `folder_delete` | [Folders](../operations/folder.md) |
| `smartFolder` | `smartFolder_list`, `smartFolder_get`, `smartFolder_create`, `smartFolder_update`, `smartFolder_delete`, `smartFolder_items` | [Smart folders](../operations/smart-folder.md) |
| `tag`, `tagGroup` | `tag_list`, `tag_get`, `tag_rename`, `tag_merge`, `tag_delete`, `tag_setColor`, `tagGroup_list`, `tagGroup_create`, `tagGroup_update`, `tagGroup_delete`, `tagGroup_addTags`, `tagGroup_removeTags` | [Tags](../operations/tag.md) · [Tag groups](../operations/tag-group.md) |
| `field`, `location` | `field_list`, `field_get`, `field_define`, `field_delete`, `field_facets`, `field_coverage`, `field_failures`, `field_usedBy`, `field_cleanup`, `field_rescan`, `location_list`, `location_add`, `location_remove`, `location_rescan`, `location_setAccess`, `location_setLook`, `location_relocate` | [Fields](../operations/field.md) · [Locations](../operations/location.md) |
| `query`, `archive`, `duplicates` | `query_validate`, `query_facets`, `archive_list`, `archive_create`, `duplicates_find`, `duplicates_resolve`, `duplicates_ignore`, `duplicates_unignore` | [Query](../operations/query.md) · [Archive](../operations/archive.md) · [Duplicates](../operations/duplicates.md) |
| `selection`, `view`, `viewer` | `selection_get`, `selection_set`, `view_get`, `view_show`, `view_setSearch`, `view_setSample`, `view_setFilters`, `view_showWith`, `viewer_open`, `viewer_close`, `viewer_current` | [Selection](../operations/selection.md) · [View](../operations/view.md) · [Viewer](../operations/viewer.md) |
| `ui`, `dialog`, `clipboard` | `ui_notify`, `ui_confirm`, `ui_prompt`, `ui_pickTags`, `ui_pickFolders`, `ui_pickField`, `ui_pickColor`, `ui_menu`, `dialog_openFile`, `dialog_openFolder`, `dialog_saveFile`, `clipboard_readText`, `clipboard_writeText`, `clipboard_copyFiles` | [UI](../operations/ui.md) · [Dialog](../operations/dialog.md) · [Clipboard](../operations/clipboard.md) |
| `job`, `tool` | `job_list`, `job_get`, `tool_list` | [Jobs](../operations/job.md) · [Tools](../operations/tool.md) |

## Live updates

Curio's MCP endpoint uses stateless JSON-RPC without push notifications. Agents waiting on background jobs (such as `duplicates_find` or queued `item_add_fromUrls` imports) can open the [SSE event stream](events.md) with the same bearer token.
