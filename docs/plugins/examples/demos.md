# Demos

The companion `demo-plugins` repository contains eight small plugins, each focused on one or two extension points. Add any folder in `demo-plugins/` via **Settings › Plugins › Add plugin** to inspect it in a running library.

## Convert pictures (`curio.convert`)

- **Folder**: `demo-plugins/convert`
- **Permissions**: `metadata.read`, `files.read`, `files.write`
- **What it shows**: Opens a standalone window (`windows[]`) from an image-only action (`where: ["item-menu", "toolbar"]`, `selection: "some"`, `kinds: ["image"]`). It decodes selected images onto `<canvas>`, converts them to PNG, JPEG or WebP and writes results through [`item.writeUrl`](../../api/operations/item-files.md#item-writeurl) (`mode: "beside"` or `mode: "replace"`).

## CSV tables (`curio.csv`)

- **Folder**: `demo-plugins/csv`
- **Permissions**: `files.read`
- **What it shows**: Registers `.csv` and `.tsv` (`kind: "document"`) with a custom `viewer.html`, a `tables` screen and a companion schema rule (`{name}.schema.json`, `{stem}.schema.json`). It publishes two `viewer_tools` (`rows` text label and `header` toggle button) and embeds `settings.html` inside **Settings › Plugins**.

## UI gallery (`curio.gallery`)

- **Folder**: `demo-plugins/gallery`
- **Permissions**: `metadata.read`, `files.read`, `ui`
- **What it shows**: Mounts `index.html` as both a standalone window (shortcut ++shift+alt+g++) and a workspace tab (`tabs[]`). It demonstrates `/curio/ui/curio.css` controls, live `curio-theme` CSS tokens and host-rendered overlays (`ui.choose`, `ui.popover`, `ui.pickTags`, `ui.pickFolders`, `ui.pickColor`, `ui.menu`, `ui.confirm` and `ui.prompt`).

## Link catcher (`curio.links`)

- **Folder**: `demo-plugins/links`
- **Permissions**: `metadata.read`, `metadata.write`, `ui`
- **What it shows**: Declares a `urls[]` handler matching GitHub, Wikipedia and ArtStation URLs and opens a review window (`opens: "catch"`). The window receives `{ handler, urls, place }` via `plugin.run` so the user can edit destination metadata before saving.

## Notes (`curio.notes`)

- **Folder**: `demo-plugins/notes`
- **Permissions**: `files.read`
- **What it shows**: Runs a hidden page extractor (`extractors[].page: "extract.html"`) on `.txt` and `.md` files. It populates two fields (`first_line` text and `lines` integer) through `curio.extractDone`.

## Review (`curio.review`)

- **Folder**: `demo-plugins/review`
- **Permissions**: `metadata.read`, `metadata.write`
- **What it shows**: Declares `review` (`single_select`: `to do`, `approved`, `redo`) and `review_note` (`text`, `block`) fields along with two Filter bar presets (`Not reviewed`, `To redo`). Its selection-aware Inspector widget (`widget.html`) writes edits back with `item.update`.

## Tag by shape (`curio.shapes`)

- **Folder**: `demo-plugins/shapes`
- **Permissions**: `metadata.read`, `metadata.write`, `exec`
- **What it shows**: Runs a Python CLI action (`run: ["python", "shapes.py"]`) from the item menu or a reusable action step (`where: ["item-menu", "step"]`, `kinds: ["image"]`). It reads dimensions through `CURIO_API_URL` and `CURIO_API_TOKEN` and tags each image `portrait`, `landscape` or `square`.

## Library stats (`curio.stats`)

- **Folder**: `demo-plugins/stats`
- **Permissions**: `metadata.read`, `ui`
- **What it shows**: Adds a workspace tab (`tabs[]`) that queries [`library.stats`](../../api/operations/libraries.md#library-stats) and charts items by kind, extension, folder, tag or month. Clicking any bar navigates the main window to those items via [`view.showWith`](../../api/operations/view.md#view-showwith) or [`view.setSearch`](../../api/operations/view.md#view-setsearch).
