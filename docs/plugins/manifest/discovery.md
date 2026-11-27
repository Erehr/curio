# Discovery

Keys inside `plugin.json`'s `filters[]`, `sorts[]` and `columns[]` arrays. See [Filters and sorts](../contributions/search.md), [List columns and Group by](../contributions/display.md) and [Appearance and theme tokens](../pages/appearance.md).

## filters[].id {#filters-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique filter preset identifier within the plugin.

## filters[].label {#filters-label}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Human-readable label shown in the Filter bar's **Presets** menu.

## filters[].query {#filters-query}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Curio [query language](../../api/query/syntax.md) expression applied when the filter preset is active (for example `"prompt IS NOT EMPTY AND steps >= 20"`).

## sorts[].label {#sorts-label}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Display label shown in the toolbar **Layout › Sort by** menu. If the backing field is currently switched off (`indexed: false`), Curio keeps the entry visible with a "(not indexed)" suffix so users know why it is inactive.

## sorts[].field {#sorts-field}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Custom field key from `fields[]` to sort by (for example `"model_triangles"` or `"read_progress"`). Only single-value sortable field types (`text`, `integer`, `float`, `boolean`, `date`, `datetime`, `single_select`, `url`, `file_ref`) produce a sort order.

## columns[].id {#columns-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique List view column identifier within the plugin.

## columns[].label {#columns-label}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Column header label shown in List view and in the column visibility picker.

## columns[].render {#columns-render}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Built-in custom cell renderer name. Currently the only supported value is `"font"`, which renders the window's live font sample string using each font item's typeface and enables the `fontsample` toolbar input.

## columns[].width {#columns-width}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null (300)</span></div>
Initial column width in pixels in List view (defaults to `300` when omitted).

## columns[].kinds {#columns-kinds}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all kinds)</span></div>
Item kinds that this custom column draws for (for example `["font"]`). Cells for other kinds stay blank.

## Example

```json
{
  "filters": [
    { "id": "high-poly", "label": "High-poly meshes", "query": "model_triangles >= 100000" },
    { "id": "rigged", "label": "Rigged models", "query": "model_animations > 0" }
  ],
  "sorts": [
    { "label": "Triangle count", "field": "model_triangles" }
  ],
  "columns": [
    { "id": "font-specimen", "label": "Specimen", "render": "font", "width": 320, "kinds": ["font"] }
  ]
}
```
