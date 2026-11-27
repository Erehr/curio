# Fields

Declare typed custom metadata fields in `plugin.json` under `fields[]`. Curio stores values in SQLite, validates edits and integrates indexed fields into the Inspector, List view, Filter bar, Query language, Smart Folders, sorting, Group by and exports.

See [Field declarations](../manifest/fields.md) for the complete key reference, defaults and validation options.

## Definition

```json
{
  "fields": [
    {
      "key": "read_state",
      "label": "Reading status",
      "type": "single_select",
      "options": ["unread", "want", "reading", "finished", "dropped"]
    },
    { "key": "pages", "label": "Pages", "type": "integer", "validation": { "min": 1 } },
    { "key": "genre", "label": "Genre", "type": "multi_select" }
  ]
}
```

Supported types are `text`, `integer`, `float`, `boolean`, `date`, `datetime`, `single_select`, `multi_select`, `json`, `url` and `file_ref`.

## Validation & UI

Curio enforces [`validation`](../manifest/fields.md#fields-validation) rules (`min`, `max`, `max_length`, `pattern` and `required`) on `item.update`, Inspector edits and plugin program replies. Set [`widget`](../manifest/fields.md#fields-widget) to choose the Inspector control: `row` for single-line text, `block` for multi-line prompts or notes, or `code` for monospace text (`multi_select` fields render as chips automatically).

## Ownership

- **Owner tracking**: Each field records its `owner` as `"user"`, `"builtin.format.<family>"` or the declaring plugin's `id`.
- **Shared fields**: When a second plugin declares an existing field `key` with the same `type`, both plugins share one field for their respective formats (for example `title` and `genre` are shared between `builtin.format.audio` and `curio.reader`). Declaring an existing `key` with a different `type` is refused.

## Indexing

Set [`indexed: false`](../manifest/fields.md#fields-indexed) on high-cardinality fields populated by an extractor (such as `seed` or `model_vertices`) to keep SQLite smaller until a user enables the field in **Settings › Fields**. Unindexed fields remain readable on demand through [`item.readFields`](../../api/operations/item-reads.md#item-readfields). Use [`visible`](../manifest/fields.md#fields-visible) and [`sort_order`](../manifest/fields.md#fields-sort_order) to control Inspector and List column visibility and ordering.

## Filters & pairs

- **`own_filter`**: Set to `true` to promote a field (such as `model` or `lora`) to a top-level Filter bar chip and standalone Smart Folder rule.
- **`filter_kind`**: Overrides the default filter control with `"list"`, `"range"`, `"text"` or `"has"`.
- **`pair`**: Binds a secondary field to a primary field (for example `lora_strength` to `lora`, or `series_index` to `series`). Indexing toggles both fields together. Sorting by the primary field tie-breaks ascending on the paired field, and multi-value pairs support `WITH` queries and inline Min/Max ranges inside the primary field's filter popover.
- **`filter_group`**: Combines multiple numeric fields owned by the same plugin (such as `model_width`, `model_depth` and `model_height`) into one multi-range filter popover under the given label.

## Usage

Every indexed field participates in:

- **Query language**: Reference by `key`, normalized label or `field.<key>` (`steps <= 12`, `lora = detail WITH lora_strength >= 0.7`).
- **Facets**: [`field.facets`](../../api/operations/field.md#field-facets) and Filter bar counts.
- **List columns, Sort by & Group by**: Single-value indexed fields appear in List columns, **Sort by**, **Group by** and Smart Folder **Automatic subfolders**. Paired secondary fields are omitted from standalone Sort and Group menus.
- **Export**: Included in JSON, CSV and `.curio.json` sidecars.

## Overrides & cleanup

- Curio records edits made by a user or via `item.update` on an extractor-backed field as manual overrides, so later rescans preserve them until the user clicks the pencil icon in the Inspector to clear the override.
- Raising `extractors[].version` triggers a one-time rescan of matching files per library.
- When a user disables an extractor-backed field or uninstalls a plugin, stored values remain inspectable and removable via [`field.cleanup`](../../api/operations/field.md#field-cleanup) in **Settings › Fields**.
