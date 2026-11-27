# Starters

Curio includes six starter templates available from **Settings › Plugins › New example plugin**. Each template creates a numbered folder (`example-<kind>`) with a unique `id`, enables it for the current library and runs with no build step.

## Window

**Files**: `plugin.json`, `index.html`, `window.js`

Opens a standalone window (`windows[]`) from an item-menu or toolbar action (`actions[].opens`) and lists the selected items' names, kinds, dimensions and file sizes. It connects with `Curio.connect()`, handles `plugin.run` and `plugin.drop`, styles controls with `/curio/ui/curio.css` and calls `selection.set` and `ui.notify`.

- **Best starting point for**: Batch tools, exporters, converters or multi-step wizards.

## Tab

**Files**: `plugin.json`, `index.html`, `tab.js`

Adds a workspace tab (`tabs[]`) beside the main grid and Tag Manager that queries top-rated pictures via `item.query` and displays their thumbnails. Clicking a tile selects it in the Inspector via `selection.set`, double-clicking opens the viewer via `viewer.open` and `plugin.beforeExit` revokes object URLs.

- **Best starting point for**: Dashboards, library analytics, visual maps or custom browsers.

## Widget

**Files**: `plugin.json`, `widget.html`, `widget.js`

Mounts a selection-aware Inspector widget (`widgets[].page` with `selection: "some"`) that listens to `plugin.selection` and shows the selected item name or count, combined size and star rating. Clicking a star writes the updated rating to all selected items with `item.update`.

- **Best starting point for**: Custom metadata panels, review controls or related-item strips.

## Viewer

**Files**: `plugin.json`, `viewer.html`, `viewer.js`

Registers a format viewer (`formats[].viewer`) for `.log` files with a dedicated `logs` screen and two `viewer_tools[]` (`lines` text readout and `problems` toggle button). It streams file text through `item.fileUrl`, highlights warnings and errors and syncs the toolbar via `viewer.setTools` on `plugin.item` and `plugin.tool`.

- **Best starting point for**: Specialized document, CAD, audio, data-table or interactive item viewers.

## Program

**Files**: `plugin.json`, `program.py`

Runs a Python action (`actions[].run`) from the item menu or a reusable action step (`"where": ["item-menu", "step"]`). It reads item IDs from `stdin`, queries sizes through `CURIO_API_URL` and `CURIO_API_TOKEN`, appends each file's size to its notes via `item.update` and prints a `{ message }` summary to `stdout`.

- **Best starting point for**: Python, Node.js or Rust CLI scripts, external tool wrappers and automated taggers.

## URL handler

**Files**: `plugin.json`, `index.html`, `links.js`

Intercepts dropped, pasted or captured `example.com` links (`urls[]` with `patterns[]` and `opens`) in a review window. It receives `{ urls, place }` through `plugin.run`, lets the user enter a note and saves the links via `item.add.fromUrls` with the destination metadata from `place`.

- **Best starting point for**: Site-specific downloaders, bookmark enrichers or gallery importers.
