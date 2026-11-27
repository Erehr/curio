# 3D models

Curio's optional **3D models** plugin (`curio.models`) renders interactive WebGL previews, generates 512 px thumbnails, pairs companion material and buffer files and indexes mesh geometry and physical dimensions.

## Install

1. Download `curio-models` from [**Erehr/curio-plugins**](https://github.com/Erehr/curio-plugins).
2. Open **Settings › Plugins**, click **Add plugin** and select the `curio-models` folder.
3. Rescan existing 3D files in **Settings › Plugins** or **Settings › Fields** to generate thumbnails and geometry metadata.

Curio enables the plugin with its requested permissions (`metadata.read`, `metadata.write`, `files.read` and `ui`) and registers six formats under the built-in **3D models** type:

| Extension | Description | Automatic companion rule |
|---|---|---|
| `.stl` | Binary and ASCII stereolithography meshes (3D printing) | - |
| `.3mf` | 3D Manufacturing Format (units and multi-object meshes) | - |
| `.ply` | Polygon File Format (vertex colors and scanned meshes) | - |
| `.glb` | Self-contained binary glTF 2.0 (materials and embedded textures) | - |
| `.obj` | Wavefront OBJ geometry | `{stem}.mtl` (materials) |
| `.gltf` | JSON glTF 2.0 scenes | `{stem}.bin` (buffers) |

Double-click any supported model to open it in the **3D models** viewer screen.

## Camera

- **Orbit**: Left-drag to rotate the camera around the model.
- **Pan**: Right-drag or middle-drag to shift the pivot point.
- **Zoom**: Scroll the mouse wheel to zoom in and out.
- **Fit to view**: Click **Fit** in the toolbar or press ++f++ to recenter and frame the model.
- **Up-axis orientation**: Open the **Appearance** toolbar popover to switch between Y-up and Z-up for CAD and 3D-printing exports.

## Shading

Control viewport overlays from the viewer toolbar or single-key shortcuts (customizable in **Settings › Shortcuts**):

| Control | Default key | Effect |
|---|---|---|
| **Fit** | ++f++ | Recenters camera and frames the bounding box |
| **Floor grid** | ++g++ | Toggles the reference floor grid beneath the model |
| **Edges** | ++e++ | Toggles wireframe edge overlay over faces |
| **Textures** | ++t++ | Toggles material image textures on or off |
| **Details** | ++i++ | Toggles the on-canvas geometry readout |
| **Next shading** | ++s++ | Cycles shading modes (smooth, flat, matcap, wireframe and more) |

The **Appearance** popover also adjusts lighting, background and shading preferences.

## Geometry

The plugin reads each supported mesh in the background and populates the **3D model** widget, a **Triangles** sort option and a **Textured models** preset in the **Filter bar**:

| Field key | Label | Type | Indexed by default | Notes |
|---|---|---|---|---|
| `model_triangles` | Triangles | Whole number | Yes | Total triangle face count; also available in **Sort by** |
| `model_width` | Width (mm) | Decimal number | Yes | Grouped under the **Model size (mm)** filter |
| `model_depth` | Depth (mm) | Decimal number | Yes | Grouped under the **Model size (mm)** filter |
| `model_height` | Height (mm) | Decimal number | Yes | Height along the standing axis; grouped under **Model size (mm)** |
| `model_unit` | Model unit | Single choice | Yes | `µm`, `mm`, `cm`, `m`, `in`, `ft` (files without declared units default to `mm`) |
| `model_textured` | Textured | Yes / No | Yes | True when materials reference image textures |
| `model_vertices` | Vertices | Whole number | No (on-demand) | Enable in **Settings › Fields** to filter or search by vertex count |
| `model_objects` | Objects | Whole number | No (on-demand) | Separate meshes or objects inside the file |

## Companions

When opening `.obj` or `.gltf` assets, Curio serves sibling files relative to the primary model. `.mtl` material files, `.bin` geometry buffers and referenced textures in the same folder load automatically in both the viewer and thumbnailer.

Because `.obj` + `.mtl` and `.gltf` + `.bin` are registered companion rules, Curio keeps them grouped as one asset during imports, folder moves, renames and exports.

## Troubleshooting

- **WebGL required**: Both the 3D viewer and hidden thumbnail generator require WebGL inside the WebView. If hardware acceleration or GPU drivers fail, thumbnails may fail to render.
- **Inspecting failures**: Check **Settings › Fields** or [`field.failures`](../../api/operations/field.md#field-failures) for corrupt or unreadable meshes. After restoring missing `.bin` or `.mtl` companions, run **More › Refresh thumbnail** or **More › Rescan metadata** from the context menu.
