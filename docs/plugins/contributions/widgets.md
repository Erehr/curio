# Widgets

Widgets place metadata summaries, interactive editors or live status readouts inside Curio's **Inspector** or **Library** sidebar.

See [Widget declarations](../manifest/widgets.md) for every key, type and default.

## Widget types

Declare widgets in `plugin.json` under `widgets[]` using either style:

=== "Field widget (declarative)"

    Renders a native Curio Inspector card listing one or more `fields[]` keys with Curio's built-in editors and click-to-filter chips for a single selected item:

    ```json
    {
      "widgets": [
        {
          "id": "ai.generation",
          "title": "AI generation",
          "icon": "sparkles",
          "fields": ["prompt", "negative_prompt", "model", "lora", "lora_strength", "sampler", "steps", "cfg", "seed"],
          "kinds": ["image", "video"]
        }
      ]
    }
    ```

=== "Page widget (custom HTML/JS)"

    Embeds a sandboxed HTML page from your plugin folder that receives live selection updates via [`plugin.selection`](../hooks/interaction.md#plugin-selection) and renders charts, cover strips, progress bars or custom controls:

    ```json
    {
      "widgets": [
        {
          "id": "curio.reader.series",
          "title": "Series",
          "icon": "books",
          "page": "web/series.html",
          "height": 160,
          "selection": "one",
          "kinds": ["archive", "document"]
        }
      ]
    }
    ```

## Identity

Prefix each widget `id` with your plugin ID (for example `curio.reader.book`) so saved layouts stay unique. Curio draws `title` as the collapsible heading above the widget; page widgets can change that heading at runtime by setting `document.title`, which `curio.js` relays to the host frame. Set `default_visible` to `false` for optional widgets that users add manually.

## Selection

Field widgets always show when a single item is selected (`"one"`). For page widgets, [`selection`](../manifest/widgets.md#widgets-selection) controls when the widget mounts and renders:

- **`"one"`**: Shown when 1 matching item is selected (default; `plugin.selection` delivers `{ count: 1, ids: [id], item }`).
- **`"some"`**: Shown when 1 or more matching items are selected.
- **`"many"`**: Shown when 2 or more matching items are selected.
- **`"any"`**: Always shown, including when nothing is selected (`count: 0`) or multiple items are selected (`count > 1`, `item: null`), suited to library-wide summaries or batch tools.

## Filtering

Set `kinds` (for example `["image", "video"]`) or `exts` (lowercase without dots, such as `["safetensors", "ckpt"]`) to hide the widget on unrelated items. Leave both empty to match all items.

## Sizing & headless

- **`headless: true`**: Omits Curio's collapsible heading so the widget sits inline like Curio's built-in Name, Notes or Rating rows.
- **Dynamic height**: `curio.js` observes `document.body` and `document.documentElement` and resizes the widget iframe up to `4000 px` to fit its content. Set `height` in `plugin.json` to supply an initial height before first measurement.
- **Hiding when empty**: When a page widget has nothing to show for the selected item (for example a book without a series), empty its body so its height collapses to `0 px`. Curio hides the empty widget frame until content returns.

## Layout

Every widget appears in **Edit layout** so users can reorder it in the Inspector, move it to the Library sidebar or place it as a popup icon button in a toolbar. Format plugins can pin widgets to a format-specific viewer screen with `formats[].screen.inspector`.

## Multi-selection

Subscribe to [`plugin.selection`](../hooks/interaction.md#plugin-selection) inside a page widget. Because `plugin.selection` is sticky, late listeners receive the current selection on subscribe:

```javascript
const curio = Curio.connect()
curio.on('plugin.selection', async ({ data }) => {
  if (!data.ids.length) {
    document.body.innerHTML = ''
    return
  }
  document.title = data.count === 1 ? 'Review' : `Review (${data.count})`
  const items = await curio.item.getMany({ ids: data.ids })
  render(items)
})
```

To show dropdown lists or rich pickers without clipping against the widget iframe bounds, use [`ui.choose`](../../api/operations/ui.md#ui-choose) and [`ui.popover`](../../api/operations/ui.md#ui-popover). See [Show controls outside a page frame](../pages/overlays.md).
