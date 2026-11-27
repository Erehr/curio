# Curio plugins

The companion [**Erehr/curio-plugins**](https://github.com/Erehr/curio-plugins) repository provides three optional plugins built exclusively on the public plugin API. Their manifests and pages serve as reference implementations for formats, viewers, hidden workers and companion grouping.

## Reader (`curio.reader`)

Reader adds ebook, PDF and comic reading with archive streaming, cover thumbnails, bibliographic extraction and series widgets. It splits high-frequency reading positions into private `plugin.itemState` while keeping searchable reading status in `fields[]`.

- **Formats**: Overrides `.cbz` and `.cb7` (`archive`) and `.pdf` (`document`); registers `.epub`, `.fb2`, `.fbz`, `.mobi`, `.azw3`, `.azw` and `.prc` under custom `"type": "Books"`.
- **Hidden workers**: Uses `thumb.html` (`thumbnail_page`, 512 px) and `extract.html` (`extractors[].page` with `comics` and `books` readers) to render covers and extract 14 fields covering reading state and progress, page and word counts, author, series with paired `series_index`, publication details, identifiers and genre.
- **Viewer chrome**: Declares a dedicated `reader` screen, 13 `viewer_tools` (including an RTL-capable `place` progress slider, `chapter` text label and tabbed `layout` and `look` popover panels) and 14 `viewer_keys`.
- **State split**: Stores page or CFI positions and bookmarks in `plugin.itemState` so updates do not trigger grid refreshes, while writing searchable `read_state` and `read_progress` to `fields[]`.
- **Widgets**: Provides three page widgets (`curio.reader.genres`, `curio.reader.book`, `curio.reader.series`) using `ui.choose` for the reading-status dropdown and `plugin.itemState.getMany` for series progress bars.

## 3D models (`curio.models`)

3D models adds a Three.js orbit viewer, hidden WebGL thumbnail rendering, companion material bundling and geometry fields. It loads relative textures and buffers through `item.fileUrl` without rewriting file paths.

- **Formats & companions**: Registers `.stl`, `.3mf`, `.ply`, `.glb`, `.obj` (companion `{stem}.mtl` as `materials`) and `.gltf` (companion `{stem}.bin` as `buffers`) under `kind: "model"`.
- **Relative file loading**: Calls `item.fileUrl` so Three.js loads sibling `.mtl`, `.bin` and texture images from Curio's host origin.
- **Fields & filters**: Declares `model_triangles` (with a `Triangles` sort), grouped physical dimensions (`model_width`, `model_depth` and `model_height` sharing `"filter_group": "Model size (mm)"`), `model_unit`, `model_textured` (with a `Textured models` filter) and unindexed `model_vertices` and `model_objects` (`"indexed": false`).
- **Viewer chrome**: Declares a `models` screen with 6 `viewer_tools` (`fit`, `grid`, `edges`, `textures`, `details`, `look`), 6 `viewer_keys` (`F`, `G`, `E`, `T`, `I`, `S`) and the declarative field widget `curio.models.info`.

## AI generation (`curio.ai-generated`)

AI generation extracts prompt and model metadata from generated images and model weight bundles without custom UI pages. It pairs multi-value LoRA tags with numeric strengths and turns double-clicking a model file into a filtered search.

- **Minimal permissions**: Requires only `["files.read"]`.
- **Paired fields**: Declares `lora` (`multi_select`, `own_filter: true`) paired with `lora_strength` (`float`, `"pair": "lora"`), enabling per-LoRA strength filters in the Filter bar and `lora = X WITH lora_strength >= 0.7` queries.
- **Model bundles**: Registers `.safetensors`, `.sft`, `.ckpt` and `.gguf` under custom `"type": "Models"` with two companion rules (`preview` with `"cover": true` and `metadata` for `.metadata.json`, `.civitai.info`, `.json` and `.txt`).
- **Open-as-filter**: Declares `"open": { "show_with": ["lora", "model"] }` so double-clicking a model item switches Curio to **All** filtered to every picture generated with that checkpoint or LoRA.
- **Named filters & sorts**: Contributes `AI-generated`, `AI models` and `≤ 12 steps` filters plus `Steps` and `CFG` sorts.
