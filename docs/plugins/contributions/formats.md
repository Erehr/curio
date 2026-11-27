# Formats

Declare format contributions under `formats[]` to register file extensions, custom display types, companion rules, thumbnails and custom viewers or open-as-search rules.

See [Format and companion declarations](../manifest/formats.md) and [Viewer screens, tools and keys](../manifest/viewers.md) for the complete key schemas.

## Extensions & type

```json
{
  "formats": [
    {
      "exts": ["epub", "fb2", "fbz", "mobi", "azw3", "azw", "prc"],
      "kind": "document",
      "type": "Books",
      "viewer": "index.html",
      "thumbnail_page": { "page": "thumb.html", "size": 512 }
    }
  ]
}
```

Use lowercase extensions without dots in `exts`, assign a built-in item `kind` (`image`, `video`, `audio`, `document`, `code`, `link`, `archive`, `font`, `model` or `other`) and optionally set `type` to display matching items under a custom label (such as `"Books"` or `"Models"`) in **Group by › Type**, the **Types** filter chip and the List **Type** column while the plugin is enabled.

## Overrides

When an enabled plugin declares an extension one of Curio's built-in formats already reads, the plugin's viewer and thumbnailer replace the built-in ones while it is enabled.

- **Settings › Plugins** displays an override indicator on the overridden plugin listing the affected extensions and overriding plugin name.
- Disabling the overriding plugin restores the built-in handler immediately.

## Viewer & screen

- **Viewer page (`viewer`)**: When the user opens a matching item, Curio mounts the viewer page in the viewer frame and keeps it loaded across adjacent items, sending [`plugin.item`](../hooks/interaction.md#plugin-item) (`{ id, kind, ext, name }`) whenever the active item changes.
- **Custom screen (`screen`)**: Registers a dedicated screen in **Edit layout** with default toolbar and Inspector widgets. Without `screen`, all formats of the plugin share one default screen named after the plugin.
- **Native toolbar controls**: Combine `screen.toolbar` with `viewer_tools[]`, `viewer_keys[]` and [`viewer.setTools`](../../api/operations/viewer.md#viewer-settools) to place native buttons, popovers, sliders and readouts in Curio's top toolbar. See [Build viewer controls and shortcuts](../pages/viewer-tools.md).

## Open as search

Instead of a `viewer` page, a format can declare `open.show_with` (used by `curio.ai-generated` for `.safetensors`, `.ckpt` and `.gguf` models):

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

Opening a matching file counts items matching the file's stem in each listed field (`lora`, `model`), picks the field with the most matches and switches the window to **All** filtered by that field and value.

## Thumbnails

Format plugins can supply thumbnails in two ways (both require `files.read`; command thumbnailers also require `exec`):

- **Hidden thumbnail page (`thumbnail_page`)**: Runs in a hidden frame while a window is open, receives [`plugin.thumbnail`](../hooks/work.md#plugin-thumbnail) requests one at a time and responds with `curio.thumbnailDone(request, picture, { width, height })` within 60 s. Suited to WebGL (3D models), `pdf.js`, `foliate-js` or Canvas rendering. See [Complete a thumbnail or extraction request](../pages/completions.md).
- **External thumbnail command (`thumbnail.command`)**: Spawns a CLI process with `{input}`, `{output}` and `{size}` placeholders (90 s timeout). See [Extractor and thumbnail program contracts](../programs/file-handlers.md).
- **Fallback**: Returning `null` from a thumbnail page or exiting non-zero from a command lets Curio's built-in thumbnailers run next.

## When disabled

When a format plugin is disabled, indexed items stay in the library under their built-in `kind` with their cached thumbnails and metadata intact. Custom `type` labels, custom viewers and companion rules pause until the plugin is re-enabled.
