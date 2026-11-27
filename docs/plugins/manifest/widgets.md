# Widgets

Keys inside `plugin.json`'s `widgets[]` array. A widget is either a **field widget** (a native Inspector section displaying declared `fields[]`) or a **page widget** (a sandboxed iframe loaded from `page`). See [Inspector and sidebar widgets](../contributions/widgets.md).

## widgets[].id {#widgets-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique widget identifier (prefix with your plugin ID, for example `"com.example.generation"` or `"com.example.review"`). Used in saved window layouts and `formats[].screen.inspector`.

## widgets[].title {#widgets-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Heading label shown at the top of the collapsible widget section and in **Edit layout**. Page widgets can update their displayed heading dynamically at runtime by setting `document.title`.

## widgets[].fields {#widgets-fields}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
List of field keys rendered natively by a field widget in the order listed. Ignored when `page` is set. Only fields that have values for the selected item (or manual plugin fields with no file reader) are shown.

## widgets[].kinds {#widgets-kinds}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all kinds)</span></div>
Item kinds the widget applies to (`"image"`, `"video"`, `"audio"`, `"document"`, `"archive"`, `"font"`, `"model"`, `"code"`, `"link"`, `"other"`). When non-empty, the widget only appears when the selected item (or every item in a multi-selection) matches one of these kinds.

## widgets[].exts {#widgets-exts}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all extensions)</span></div>
Lowercase file extensions without leading dots (for example `["png", "webp", "safetensors"]`). When non-empty, restricts the widget to items matching those extensions.

## widgets[].default_visible {#widgets-default_visible}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: true</span></div>
Whether Curio places the widget in the default Inspector layout when the plugin is enabled. Set to `false` for optional widgets that users add manually in **Edit layout**.

## widgets[].page {#widgets-page}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Relative path to an HTML page inside the plugin folder (for example `"ui/widget.html"`). When set, Curio renders the widget as a sandboxed iframe under the plugin's origin (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows) instead of a field list, delivering selection snapshots through [`plugin.selection`](../hooks/interaction.md#plugin-selection).

## widgets[].height {#widgets-height}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null (240)</span></div>
Initial iframe height in pixels for a page widget (defaults to `240` px when omitted) before `curio.js` reports the page's measured content height. Pages that load `/curio/ui/curio.js` report their height automatically via `ResizeObserver` and `MutationObserver` (up to `4000` px; reporting `0` px hides the widget until content appears) until the user drags the widget's bottom edge to set a manual height.

## widgets[].selection {#widgets-selection}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: "one"</span></div>
Controls when a **page widget** is shown based on the current selection count:

| Value | When shown |
|---|---|
| `"one"` | Exactly 1 item selected (default; also used for any unrecognized value). |
| `"many"` | 2 or more items selected. |
| `"some"` | 1 or more items selected. |
| `"any"` | Always shown, including when nothing is selected. |

Field widgets always require a single selected item (`"one"`).

## widgets[].icon {#widgets-icon}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null ("puzzle")</span></div>
Tabler icon name used in **Edit layout** and as the widget's popup launcher button when the widget is placed in a toolbar or compact sidebar.

## widgets[].headless {#widgets-headless}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: false</span></div>
When `true`, renders the widget without its collapsible heading bar (matching Curio's built-in Name and Notes widgets). A headless widget cannot be folded by clicking a header.

## Example

```json
{
  "widgets": [
    {
      "id": "generation",
      "title": "Generation",
      "icon": "sparkles",
      "kinds": ["image"],
      "fields": ["checkpoint", "sampler", "steps", "cfg", "seed", "prompt"]
    },
    {
      "id": "review",
      "title": "Review",
      "icon": "checklist",
      "page": "ui/review.html",
      "height": 180,
      "selection": "some",
      "default_visible": true,
      "headless": false
    }
  ]
}
```
