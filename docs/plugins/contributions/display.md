# Display

Plugins can add List view columns, register custom display type labels and supply indexed fields for **Group by** headers.

See [Search, sort and column declarations](../manifest/discovery.md#columns-id) and [Format and companion declarations](../manifest/formats.md#formats-type) for the full key schemas.

## Field columns

Every visible indexed field declared in `fields[]` appears in:

- The **List view** column picker (`+` at the end of column headers)
- Column-header click sorting for single-value sortable fields
- Smart Folder **Automatic subfolders** (`field:<key>`)

## Custom columns

When a List view column needs a specialized host renderer instead of formatted field text, declare it in `columns[]`:

```json
{
  "columns": [
    {
      "id": "font_sample",
      "label": "Font sample",
      "render": "font",
      "width": 260,
      "kinds": ["font"]
    }
  ]
}
```

Curio currently supports the `"font"` renderer, which draws the window's active sample text (set via [`view.setSample`](../../api/operations/view.md#view-setsample)) in each font item's typeface. See [columns[]](../manifest/discovery.md#columns-id) for width and kind filtering keys.

## Custom types

Register a custom display type label with `formats[].type`:

```json
{
  "formats": [
    {
      "exts": ["epub", "fb2", "fbz", "mobi", "azw3", "azw", "prc"],
      "kind": "document",
      "type": "Books",
      "viewer": "index.html"
    }
  ]
}
```

While the plugin is enabled, matching items appear under **Books** (or **Models**, etc.) in:

- **Layout › Group by › Type**
- The **Types** Filter bar popover
- The List view **Type** column

Their underlying `Item.kind` stays a built-in item kind (such as `"document"` or `"other"`), so API queries and built-in fallbacks continue working when the plugin is disabled.

## Group by field

Every indexed single-value custom field appears in **Layout › Group by** across Grid, Masonry, Justified and List views (Spectrum does not group items). Paired secondary fields (fields with `pair` set) are omitted from standalone **Sort by** and **Group by** menus because they sort as tie-breakers beneath their primary field.

## Display vs companions

Curio has three separate grouping mechanisms:

| Goal | Mechanism | Manifest key |
|---|---|---|
| Give extensions a custom category name in **Group by › Type** and the **Types** filter | Custom display type | `formats[].type` |
| Group results into visual collapsible sections by metadata value | Indexed single-value field | `fields[]` (`indexed: true`) |
| Collapse multiple related files on disk into **one item** | Companion rule | `formats[].companions[]` (see [Register files that form one item](companions.md)) |
