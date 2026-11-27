# Formats

Keys inside `plugin.json`'s `formats[]` array. See [Formats, viewers and thumbnailers](../contributions/formats.md) and [Companion files](../contributions/companions.md).

## formats[].exts {#formats-exts}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge">Required</span></div>
Lowercase file extensions without leading dots (for example `["epub", "cbz", "cbr"]` or `["gltf", "glb", "stl", "3mf"]`).

## formats[].kind {#formats-kind}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Built-in item kind assigned to matching files when scanned: `"image"`, `"video"`, `"audio"`, `"document"`, `"archive"`, `"font"`, `"model"`, `"code"`, `"link"` or `"other"`. Leave empty if only adding companions or overriding a viewer for an already-classified extension.

## formats[].type {#formats-type}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Optional custom display type label shown in **Group by › Type**, the **Types** filter popover and the List view **Type** column (for example `"Books"` or `"Models"`).

## formats[].thumbnail {#formats-thumbnail}
<div class="curio-meta"><span class="curio-badge">object | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
External command thumbnailer configuration (`{ "command": [...] }`). Runs in the background thumbnail worker without requiring an open window. Requires the `files.read` and `exec` permissions.

## formats[].viewer {#formats-viewer}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Relative path to an HTML page inside the plugin folder (served from `plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows) that replaces Curio's built-in viewer whenever a matching file is opened. Receives [`plugin.item`](../hooks/interaction.md#plugin-item) and [`plugin.tool`](../hooks/interaction.md#plugin-tool).

## formats[].thumbnail_page {#formats-thumbnail_page}
<div class="curio-meta"><span class="curio-badge">object | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Browser-rendered thumbnailer configuration (`{ "page": "...", "size": 512 }`). Runs in a hidden page inside the open window (useful for WebGL/Three.js, Canvas or foliate-js rendering) and receives [`plugin.thumbnail`](../hooks/work.md#plugin-thumbnail). Requires the `files.read` permission.

## formats[].screen {#formats-screen}
<div class="curio-meta"><span class="curio-badge">object | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Custom **Edit layout** screen for files shown in this format's `viewer` page. When omitted, formats with a viewer share the plugin's first declared screen, or an auto-named screen if none is declared. See [Viewer screens, tools and keys](viewers.md).

## formats[].companions {#formats-companions}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Declarative rules for sibling files in the same directory that belong to the primary file and are grouped into one multi-file item during scanning and import.

## formats[].open {#formats-open}
<div class="curio-meta"><span class="curio-badge">object | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Alternative open behavior for files that are references rather than media to view (such as AI model checkpoint `.safetensors` files).

## formats[].thumbnail.command {#formats-thumbnail-command}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge">Required (inside thumbnail)</span></div>
Argv array executed to generate a thumbnail. Substitutes `{input}` (source file path), `{output}` (destination image path to write) and `{size}` (target maximum edge in pixels). Relative executable paths resolve against the plugin folder.

## formats[].thumbnail_page.page {#formats-thumbnail_page-page}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required (inside thumbnail_page)</span></div>
Relative path to the HTML page inside the plugin folder that renders thumbnails on demand via [`plugin.thumbnail`](../hooks/work.md#plugin-thumbnail).

## formats[].thumbnail_page.size {#formats-thumbnail_page-size}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null (512)</span></div>
Requested render edge size in pixels passed to the thumbnail page (defaults to `512`, clamped to `64`–`2048`).

## formats[].open.show_with {#formats-open-show_with}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
List of custom field keys. When the user opens a matching file, Curio checks which of these fields reference the file's stem on other items (the field with the most matching items wins) and navigates **All** filtered to that field value (for example, opening a `.safetensors` LoRA file filters **All** to items matching `lora = "<name>"`).

## formats[].companions[].role {#formats-companions-role}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Semantic role assigned to grouped companion files (for example `"preview"`, `"metadata"`, `"live"` or `"raw"`), using ASCII letters, digits, `-` or `_` (up to 40 characters). Also referenced by `extractors[].json.role`.

## formats[].companions[].files {#formats-companions-files}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge">Required</span></div>
Filename templates checked in the primary file's directory in order of preference. Each template must begin with `{stem}` (primary filename without extension) or `{name}` (full primary filename including extension), followed by literal text (for example `["{stem}.preview.png", "{stem}.png", "{name}.json"]`). Exact literal filenames without `{stem}` or `{name}` are not supported.

## formats[].companions[].cover {#formats-companions-cover}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: false</span></div>
When `true`, Curio generates the grouped item's thumbnail from the first matching companion file instead of the primary file.

## formats[].companions[].verify {#formats-companions-verify}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Optional built-in content verification check the primary and companion must pass before grouping. Currently supports `"live-photo"` (verifies Apple Live Photo content identifier metadata matching between photo and `.mov`).

## Example

```json
{
  "formats": [
    {
      "exts": ["safetensors", "ckpt"],
      "kind": "other",
      "type": "AI Models",
      "open": {
        "show_with": ["checkpoint", "lora"]
      },
      "companions": [
        {
          "role": "preview",
          "files": ["{stem}.preview.png", "{stem}.preview.jpg", "{stem}.png"],
          "cover": true
        },
        {
          "role": "metadata",
          "files": ["{stem}.civitai.info", "{stem}.json"]
        }
      ]
    },
    {
      "exts": ["glb", "gltf", "stl"],
      "kind": "model",
      "type": "3D Models",
      "viewer": "ui/viewer.html",
      "thumbnail_page": { "page": "ui/thumb.html", "size": 512 }
    }
  ]
}
```
