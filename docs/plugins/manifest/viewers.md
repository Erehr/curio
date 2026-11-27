# Viewers

Keys inside `formats[].screen`, `viewer_tools[]` and `viewer_keys[]`. See [Viewer toolbar controls and keys](../pages/viewer-tools.md).

## formats[].screen.id {#formats-screen-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique screen identifier within the plugin. Curio registers the layout screen under the composite key `<plugin>/<id>` before the fallback `other` viewer screen.

## formats[].screen.title {#formats-screen-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Screen display name shown in **Edit layout**'s screen selector (for example `"3D Models"` or `"Books"`).

## formats[].screen.icon {#formats-screen-icon}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null ("file")</span></div>
Tabler icon name shown beside the screen title in **Edit layout**.

## formats[].screen.toolbar {#formats-screen-toolbar}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] (all tools)</span></div>
Ordered list of default toolbar entries for this screen, mixing Curio's built-in viewer widgets (such as `"v.back"`, `"v.count"`, `"v.group"`, `"breadcrumb"`, `"v.reveal"`, `"v.open"`, `"v.strip"`, `"nav"`, `"spacer"`) and bare `viewer_tools[].id` values from this plugin (for example `"wireframe"`, `"fov"`, `"stats"`). Do not prefix tool IDs with `"tool:<plugin>/"` here; Curio resolves bare IDs against this plugin's `viewer_tools[]` automatically. When empty, Curio places all of the plugin's `viewer_tools[]` in the standard viewer toolbar.

## formats[].screen.inspector {#formats-screen-inspector}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Additional widget IDs appended to the default Inspector layout below Curio's standard item widgets whenever this viewer screen is active. Accepts plugin widget IDs from `widgets[]` and Curio built-in widget IDs.

## viewer_tools[].id {#viewer_tools-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique control identifier within the plugin. Referenced by bare `id` in `formats[].screen.toolbar`, stored in saved layouts as `tool:<plugin>/<id>`, populated at runtime by [`viewer.setTools`](../../api/operations/viewer.md#viewer-settools) and dispatched back to the viewer page via [`plugin.tool`](../hooks/interaction.md#plugin-tool).

## viewer_tools[].title {#viewer_tools-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Label shown in **Edit layout** and as the fallback tooltip before the viewer page updates the tool dynamically.

## viewer_tools[].icon {#viewer_tools-icon}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null ("puzzle")</span></div>
Tabler icon name shown in **Edit layout** and on button controls when no dynamic icon override is supplied.

## viewer_tools[].kind {#viewer_tools-kind}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: "button"</span></div>
Control presentation shape used by the toolbar and **Edit layout**:

| Value | Behavior |
|---|---|
| `"button"` | Clickable icon or toggle button. Default when omitted. |
| `"slider"` | Inline range slider (for example camera FOV, zoom or page scrubber). |
| `"text"` | Inline readout text (for example current page `"12 / 240"` or triangle count). |

## viewer_keys[].id {#viewer_keys-id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique key action identifier within the plugin. Sent as `id` in [`plugin.tool`](../hooks/interaction.md#plugin-tool) whenever the user presses the shortcut while the viewer page is active (and either `viewer.setTools` omitted `keys` or included this `id`).

## viewer_keys[].title {#viewer_keys-title}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Human-readable action name listed under **Settings › Shortcuts** so users can inspect or rebind the shortcut.

## viewer_keys[].key {#viewer_keys-key}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null (unbound)</span></div>
Default key binding string (for example `"Shift+]"` or `"W"`). Users can override or clear it in **Settings › Shortcuts**.

## Example

```json
{
  "formats": [
    {
      "exts": ["glb", "gltf", "obj", "stl"],
      "kind": "model",
      "viewer": "ui/viewer.html",
      "screen": {
        "id": "model",
        "title": "3D Model",
        "icon": "cube",
        "toolbar": ["v.back", "v.count", "spacer", "wireframe", "fov", "stats", "v.open"],
        "inspector": ["model_info"]
      }
    }
  ],
  "viewer_tools": [
    { "id": "wireframe", "title": "Wireframe", "icon": "grid-dots", "kind": "button" },
    { "id": "fov", "title": "Field of view", "icon": "eye", "kind": "slider" },
    { "id": "stats", "title": "Mesh stats", "kind": "text" }
  ],
  "viewer_keys": [
    { "id": "wireframe", "title": "Toggle wireframe", "key": "W" },
    { "id": "reset-camera", "title": "Reset camera", "key": "R" }
  ]
}
```
