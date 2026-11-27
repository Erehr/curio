# Search

Plugins can add saved queries, dedicated filter chips, grouped numeric ranges, custom sorts, sidebar query panels and click-to-filter navigation declaratively in `plugin.json`.

See [Search, sort and column declarations](../manifest/discovery.md) and [Window, tab and panel declarations](../manifest/surfaces.md#panels-id) for the full key schemas.

## Filters

Declare reusable one-click queries under `filters[]`:

```json
{
  "filters": [
    { "id": "ai.any", "label": "AI-generated", "query": "software IS NOT EMPTY" },
    { "id": "ai.models", "label": "AI models", "query": "ext IN (safetensors, sft, ckpt, gguf)" },
    { "id": "ai.fast", "label": "≤ 12 steps", "query": "steps <= 12" }
  ]
}
```

Each entry appears in the **Filter bar** menu, combines via `AND` with other active filters and can be pinned to the bar.

## Field filters

- **Promoted filters**: Set `"own_filter": true` on a field in `fields[]` to place it beside Curio's main filter chips and top-level Smart Folder rules.
- **Grouped numeric ranges**: Set `"filter_group": "Model size (mm)"` on multiple numeric fields of the same plugin (`model_width`, `model_depth`, `model_height`) to combine their Min/Max inputs in one Filter bar popover.
- **Paired ranges**: Set `"pair": "lora"` on a numeric field (`lora_strength`) paired with a multi-value field (`lora`) so users can filter value and strength ranges together inside the parent popover, compiling to a `WITH` expression.

## Shared queries

All plugin filter queries use Curio's [Query language](../../api/query/syntax.md). When a user turns off indexing for a field referenced by a plugin filter or sort, [`field.usedBy`](../../api/operations/field.md#field-usedby) reports your filter or sort label so the user sees what will stop working.

## Sorts

Add named entries to Curio's **Sort by** menu using `sorts[]`:

```json
{
  "sorts": [
    { "label": "Steps", "field": "steps" },
    { "label": "CFG", "field": "cfg" }
  ]
}
```

The `field` key must reference an indexed custom or plugin field. If the user disables indexing on that field, sorting falls back to date added.

## Panels

Add a declarative panel to the **Library** sidebar using `panels[]`:

```json
{
  "panels": [
    {
      "id": "quick-views",
      "title": "AI quick views",
      "text": "Common generation filters.",
      "filters": [
        { "id": "sdxl", "label": "SDXL renders", "query": "base_model = SDXL" },
        { "id": "high_steps", "label": "High steps (≥ 30)", "query": "steps >= 30" }
      ]
    }
  ]
}
```

Users can move, collapse or hide declarative sidebar panels in **Edit layout** alongside built-in sidebar sections.

## Open filtered

Two mechanisms navigate the main window to a live filtered view:

- **Programmatically from a page or program**: Call [`view.showWith`](../../api/operations/view.md#view-showwith) with `{ tag }` or `{ field, value }` to show **All** filtered by that exact tag or custom field value, displaying the active filter chip so the user can refine or clear it.
- **Declaratively on file open (`formats[].open.show_with`)**: For reference items that open as a search rather than a viewer (such as AI model weights `.safetensors`, `.ckpt` or `.gguf`), declare `open.show_with` in `formats[]` (see [Format and companion declarations](../manifest/formats.md#formats-open-show_with)):

```json
{
  "formats": [
    {
      "exts": ["safetensors", "sft", "ckpt", "gguf"],
      "kind": "other",
      "type": "Models",
      "open": { "show_with": ["lora", "model"] }
    }
  ]
}
```

Double-clicking a matching file checks the fields listed in `show_with`, picks the field where more items match the file's name and opens **All** filtered to those matching items.
