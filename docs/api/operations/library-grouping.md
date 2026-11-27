# library.grouping

Operations in the `library.grouping` namespace inspect, dry-run, apply and revert companion-file grouping rules across a library. Built-in rules (such as RAW+JPEG pairs, Live Photos and XMP sidecars) and plugin-declared companion rules automatically group related files into single compound items. To group, split or detach individual items manually, see [Group files into items](./item-grouping.md).

## library.grouping {#library-grouping}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/grouping</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_grouping</span>
</div>

Lists the companion grouping rules currently active in the library in evaluation order, along with how many non-trashed member files each rule currently groups. Any inactive rule IDs and manual groups created by [`item.group`](./item-grouping.md#item-group) (`id: "user"`) are appended at the end whenever they still own at least one grouped member file, without `exts` or `files`.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `rules` | `object[]` | Array of rule entries (see fields below). |
| `rules[].id` | `string` | Rule identifier (for example `"library/raw-pairs"`, `"<plugin-id>/<role>"` or `"user"` for manual groups). |
| `rules[].owner` | `string` | Owner prefix before the last `/` (for example `"library"` or a plugin ID; `""` for `"user"`). |
| `rules[].role` | `string` | Role name assigned to grouped members (for example `"raw-pairs"`, `"sidecar"` or `"user"`). |
| `rules[].grouped` | `number` | Number of non-trashed member files currently grouped under this rule ID. |
| `rules[].exts` | `string[]` | Present on active rules only: lowercase primary file extensions the rule matches. |
| `rules[].files` | `string[]` | Present on active rules only: companion filename templates (for example `"{stem}.xmp"` or `"{name}.json"`). |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/grouping \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const { rules } = await curio.library.grouping();
    for (const rule of rules) {
      console.log(rule.id, rule.grouped);
    }
    ```

=== "MCP"

    ```json
    {
      "name": "library_grouping",
      "arguments": {}
    }
    ```

**Example response**

```json
{
  "rules": [
    {
      "id": "library/raw-pairs",
      "owner": "library",
      "role": "raw-pairs",
      "grouped": 48,
      "exts": ["cr2", "cr3", "nef", "arw", "dng", "raf"],
      "files": ["{stem}.jpg", "{stem}.jpeg", "{stem}.heic"]
    },
    {
      "id": "user",
      "owner": "",
      "role": "user",
      "grouped": 6
    }
  ]
}
```

**Side effects, events and errors**

- Emits no public events and modifies no items.
- Errors: `unknown_library`, `library_closed`, `library_offline`, `not_allowed` (missing `metadata.read`).
- Related: [`library.grouping.survey`](#library-grouping-survey), [`library.grouping.apply`](#library-grouping-apply), [`library.grouping.split`](#library-grouping-split).

---

## library.grouping.survey {#library-grouping-survey}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/grouping/survey</span>
  <span class="curio-badge">Permission: metadata.read</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_grouping_survey</span>
</div>

Performs a read-only dry run of all active grouping rules against the files currently in the library and reports how many standalone files each rule would group. Nothing in the library is modified. Because surveying inspects files across locations, it runs on the library's scanner thread and waits up to 120 seconds.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers. |

**Returns**

An inline JSON object:

| Field | Type | Description |
|---|---|---|
| `rules` | `{ id: string; count: number }[]` | Active rule IDs paired with the number of existing ungrouped files each rule would attach as members. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/grouping/survey \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{}'
    ```

=== "JavaScript"

    ```js
    const { rules } = await curio.library.grouping.survey();
    ```

=== "MCP"

    ```json
    {
      "name": "library_grouping_survey",
      "arguments": {}
    }
    ```

**Example response**

```json
{
  "rules": [
    { "id": "library/raw-pairs", "count": 14 },
    { "id": "library/sidecars", "count": 32 }
  ]
}
```

**Side effects, events and errors**

- Emits no public events and modifies no state.
- Errors: `unknown_library`, `library_closed`, `library_offline`, `failed` (timeout after 120 seconds or library closed mid-survey), `not_allowed` (missing `metadata.read`).
- Related: [`library.grouping`](#library-grouping), [`library.grouping.apply`](#library-grouping-apply).

---

## library.grouping.apply {#library-grouping-apply}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/grouping/apply</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_grouping_apply</span>
</div>

Applies active companion grouping rules (or only the subset named in `rules`) to files already in the library. Files that were previously split apart manually (marked solo) are skipped unless re-enabled with [`item.allowGrouping`](./item-grouping.md#item-allowgrouping). Work runs on the library's scanner thread; if it finishes within 120 seconds, the response returns `{ count }`, and if it takes longer, the call returns `{ queued: true }` while grouping continues in the background.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `rules` | `string[]` | No | All active rules | Optional list of rule IDs (from [`library.grouping`](#library-grouping)) to apply. Unknown or inactive rule IDs are ignored. |
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers. |

**Returns**

A union of two inline JSON objects:

| Shape | Field | Type | Description |
|---|---|---|---|
| Completed | `count` | `number` | Number of member files newly grouped into parent items. |
| Timed out (> 120s) | `queued` | `true` | Returned when grouping is still running in the background. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/grouping/apply \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"rules": ["library/raw-pairs"]}'
    ```

=== "JavaScript"

    ```js
    const result = await curio.library.grouping.apply({ rules: ['library/raw-pairs'] });
    ```

=== "MCP"

    ```json
    {
      "name": "library_grouping_apply",
      "arguments": {
        "rules": ["library/raw-pairs"]
      }
    }
    ```

**Example response**

```json
{
  "count": 14
}
```

If grouping exceeds 120 seconds:

```json
{
  "queued": true
}
```

**Side effects, events and errors**

- Merges each grouped member's tags, folders, missing rating/notes/URL and user fields into its parent item while preserving the member's original metadata for a future split.
- Events: [`items.changed`](../events/items.md#items-changed) (`what: "metadata"`) for affected parent items, and [`items.removed`](../events/items.md#items-removed) (`purged: false`) for member files that leave the top-level item list.
- Errors: `unknown_library`, `library_closed`, `library_offline`, `failed` (library closed before completion), `not_allowed` (missing `metadata.write`).
- Related: [`library.grouping.survey`](#library-grouping-survey), [`library.grouping.split`](#library-grouping-split), [`item.allowGrouping`](./item-grouping.md#item-allowgrouping).

---

## library.grouping.split {#library-grouping-split}

<div class="curio-meta">
  <span class="curio-badge">POST /api/v1/library/grouping/split</span>
  <span class="curio-badge">Permission: metadata.write</span>
  <span class="curio-badge curio-badge--muted">Target: Library</span>
  <span class="curio-badge curio-badge--muted">Callers: Web, MCP, Plugin</span>
  <span class="curio-badge">MCP: library_grouping_split</span>
</div>

Splits off every member file grouped by a specific rule ID, restoring each file as a standalone item with its original metadata plus the parent item's current folders. Members grouped into the same parent items by other rules remain attached.

Unlike [`item.split`](./item-grouping.md#item-split), files released by `library.grouping.split` are not marked solo, so active rules may group them again later. If splitting takes more than 120 seconds, the call returns `{ queued: true }` while work continues in the background.

**Parameters**

| Parameter | Type | Required | Default | Description |
|---|---|---|---|---|
| `rule` | `string` | Yes | — | Rule ID (as listed by [`library.grouping`](#library-grouping), including inactive rule IDs or `"user"`). |
| `library` | `string` | No | Default library | Routing parameter: library ID, folder path or unique name. Not allowed for plugin callers. |

**Returns**

A union of two inline JSON objects:

| Shape | Field | Type | Description |
|---|---|---|---|
| Completed | `count` | `number` | Number of member files released back into standalone items. |
| Timed out (> 120s) | `queued` | `true` | Returned when splitting is still running in the background. |

**Example request**

=== "HTTP"

    ```bash
    curl -s http://127.0.0.1:41733/api/v1/library/grouping/split \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"rule": "library/raw-pairs"}'
    ```

=== "JavaScript"

    ```js
    const result = await curio.library.grouping.split({ rule: 'library/raw-pairs' });
    ```

=== "MCP"

    ```json
    {
      "name": "library_grouping_split",
      "arguments": {
        "rule": "library/raw-pairs"
      }
    }
    ```

**Example response**

```json
{
  "count": 48
}
```

If splitting exceeds 120 seconds:

```json
{
  "queued": true
}
```

**Side effects, events and errors**

- Detaches every member grouped by `rule`, restores its saved tags, folders, fields, notes and rating, invalidates the parent thumbnail if a released member acted as cover and queues managed-store synchronization for affected items.
- Events: [`items.changed`](../events/items.md#items-changed) (`what: "metadata"`) for affected parent items, and [`items.added`](../events/items.md#items-added) (with `how: "split"`) for the newly standalone items.
- Errors: `bad_request` (missing `rule`), `unknown_library`, `library_closed`, `library_offline`, `failed` (library closed before completion), `not_allowed` (missing `metadata.write`).
- Related: [`library.grouping`](#library-grouping), [`library.grouping.apply`](#library-grouping-apply), [`item.split`](./item-grouping.md#item-split).
