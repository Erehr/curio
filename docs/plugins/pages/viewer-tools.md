# Viewer tools

Format viewer pages (`formats[].viewer`) render content while delegating toolbar buttons, dropdown settings panels, progress sliders, text readouts and rebindable keyboard shortcuts to Curio's native toolbar.

## Screens

Declare a `screen` on your format in `plugin.json` so users can customize your viewer's toolbar and Inspector independently in **Edit layout**:

```json
{
  "formats": [
    {
      "exts": ["stl", "3mf", "ply", "glb"],
      "kind": "model",
      "viewer": "index.html",
      "screen": {
        "id": "models",
        "title": "3D models",
        "icon": "cube",
        "toolbar": ["v.back", "v.count", "v.group", "breadcrumb", "spacer", "fit", "grid", "edges", "textures", "details", "look", "v.strip", "nav"],
        "inspector": ["curio.models.info"]
      }
    }
  ]
}
```

In `screen.toolbar`, reference your plugin's `viewer_tools[]` by their bare `id` (such as `"fit"` or `"wireframe"`, not `"tool:..."`). Curio qualifies matching tool IDs internally and resolves all other names against Curio's built-in toolbar widgets (`v.back`, `spacer`, `nav` and similar).

## Declarations

Register each toolbar widget in `viewer_tools[]` (`kind`: `"button"` default, `"slider"` or `"text"`) and each viewer-scoped shortcut in `viewer_keys[]`:

```json
{
  "viewer_tools": [
    { "id": "place", "title": "Progress", "icon": "progress", "kind": "slider" },
    { "id": "chapter", "title": "Chapter", "icon": "list", "kind": "text" },
    { "id": "fit", "title": "Fit", "icon": "focus-centered", "kind": "button" },
    { "id": "look", "title": "Appearance", "icon": "adjustments-horizontal", "kind": "button" }
  ],
  "viewer_keys": [
    { "id": "fit", "title": "Fit", "key": "F" },
    { "id": "shading", "title": "Next shading", "key": "S" }
  ]
}
```

Viewer keys appear in their own group in **Settings › Shortcuts** (for example *3D models viewer* or *Reader viewer*), activate only while your viewer page is shown and take precedence over global shortcuts.

## viewer.setTools

Call [`viewer.setTools`](../../api/operations/viewer.md#viewer-settools) whenever the open item or viewer state changes (max 12 buttons, 12 labels, 1 slider, 64 keys):

```javascript
await curio.viewer.setTools({
  labels: [{ id: 'chapter', text: 'Page 14 / 48' }],
  slider: { id: 'place', label: 'Page', min: 1, max: 48, step: 1, value: 14, dir: 'ltr' },
  buttons: [
    { id: 'fit', label: 'Fit to view', icon: 'focus-centered', on: false },
    { id: 'look', label: 'Appearance', icon: 'adjustments-horizontal', panel: buildAppearancePanel() },
  ],
  keys: ['fit', 'shading'],
})
```

Tooltips on toolbar buttons append the user's current shortcut binding on their own, so omit key names from `label`.

## Controls

- **Buttons**: Set `on: true` to highlight active toggle buttons (such as Double page or Floor grid).
- **Labels**: Drawn wherever the matching `kind: "text"` `viewer_tools` widget is placed.
- **Slider**: Set `dir: 'rtl'` for right-to-left books or manga so progress runs right-to-left. While the user drags the slider, Curio delivers `{ id, value }` via `plugin.tool` without letting intermediate `setTools` updates snap the handle back under the pointer.

## Panels

Attach a `panel` array to any button to open a native Curio popover instead of emitting a bare click. Pass either a flat list of up to 40 controls or up to 6 tabs (`[{ tab: "Page", icon: "book", controls: [...] }]`):

- **`{ type: "section", label: "Typography" }`**: Section heading.
- **`{ type: "separator" }`**: Horizontal rule.

## Inputs

| Control `type` | Extra properties | Emitted `plugin.tool` `value` |
|---|---|---|
| `slider` | `min`, `max`, `step`, `value`, `unit` (max 8 chars, e.g. `"%"`, `"px"`) | `number` |
| `toggle` | `value: boolean` | `boolean` |
| `choice` | `value`, `options: [{ value, label, icon? }]` (max 12, rendered as segmented buttons) | Selected `option.value` |
| `select` | `value`, `options: [{ value, label, icon? }]` (max 500, rendered as dropdown) | Selected `option.value` |
| `swatches` | `value`, `options: [{ value, label, background, foreground?, icon? }]` (max 12 color dots) | Selected `option.value` |

## Events

Every toolbar interaction arrives at your viewer page via [`plugin.tool`](../hooks/interaction.md#plugin-tool):

```javascript
curio.on('plugin.tool', async ({ data }) => {
  if (data.menu) {
    // Right-click or long-press on button: ui.menu opens anchored beneath the button
    const { id } = await curio.ui.menu({ items: [{ id: 'reset', label: 'Reset view' }] })
    if (id === 'reset') resetCamera()
    return
  }
  if (data.value !== undefined) {
    applyControlChange(data.id, data.value)
  } else {
    // Button click or viewer_key press (data.key === true)
    handleCommand(data.id)
  }
  await syncTools()
})
```

## Cleanup

Call `curio.viewer.setTools({})` to clear controls if an unsupported or broken item loads. Curio also clears `viewerTools` when your viewer iframe reloads or detaches.
