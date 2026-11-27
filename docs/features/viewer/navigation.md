# Navigation

Open assets inside the main window, pop them out into separate windows or hand them off to external apps. While an asset is open, step through the current result list, browse the filmstrip or cycle through grouped files inside one asset.

## Opening

Open any selected asset in three ways:

- **Open in the current window**: Double-click an asset (default in **Settings › Controls › Double-click**), press ++enter++ or press ++space++ (when **Space** is set to **Toggle viewer**). The asset fills the central stage while keeping your viewer screen's sidebar, toolbar and Inspector layout.
- **Open in a window**: Middle-click an asset in the grid (default in **Settings › Controls › Middle click**), press ++shift+alt+w++ or choose **Open in a window** from the context menu. Separate windows stay open side by side while you browse in the main window.
- **Open with default app**: Press ++shift+alt+o++, choose **Open with default app** from the context menu or middle-click the open viewer stage. You can also set **Double-click** or **Middle click** in **Settings › Controls** to open files directly in the default app.

Some plugins define custom open behavior for specific file types, such as AI model weights opening matching generated images. See [Inspect AI generation metadata](ai.md).

!!! note "Missing and offline files"
    If an asset's file is missing or on an offline drive, Curio shows a warning instead of opening a blank viewer. Stepping through results with arrow keys or the mouse wheel skips missing and offline assets automatically.

## Stepping

While the viewer is open, Curio steps through the exact ordered list shown in your active folder, search, filters and sort order:

- Use the **Previous item** and **Next item** toolbar arrows to step through assets. The **Position** readout (`3 / 120`) shows your place in the results.
- Press ++left++ / ++right++ (or ++up++ / ++down++) to step to the previous or next asset.
- Stepping updates the library selection so the Inspector reflects the asset you are viewing.

Curio preloads the previous asset and the next two assets in the background after you pause on an asset. Stepping past assets cancels pending decodes for skipped files.

## Filmstrip

The filmstrip shows a scrollable ribbon of neighboring thumbnails from your current result set so you can jump to another asset without returning to the grid.

- **Toggle visibility**: Click **Strip** in the viewer toolbar. The filmstrip hides automatically in fullscreen mode (++f11++) and in separate asset windows.
- **Change position**: Right-click the filmstrip and choose **Left**, **Below** or **Right**, or configure **Settings › Preview › Image viewer › Strip**.
- **Scroll and jump**: Scroll the mouse wheel over the filmstrip to pan along the strip (even when docked below), and click any thumbnail to jump to that asset.

The filmstrip keeps the active asset centered with the selection border.

## Grouped files

When you combine multiple assets from one location with **Group as one item**, Curio treats them as one library asset while letting you inspect every file inside the viewer:

- The viewer opens on the primary file first, followed by manually grouped files sorted naturally by name.
- Press ++page-up++ and ++page-down++, or use the **Files** stepper widget (`1 / N`) in the viewer toolbar, to move between files inside the asset.
- Each file uses the viewer for its format. Stepping across an image, video, text file and PDF switches viewers while the Inspector stays focused on the parent asset.
- The toolbar breadcrumb shows the parent asset followed by the active grouped file; click the parent crumb to return to the primary file.

Files grouped automatically by rules (Live Photo `.mov` clips, RAW + JPEG partners, `.xmp`/`.aae` sidecars and AI model previews) do not appear in the manual file stepper. They use dedicated controls instead, such as **Live Photo** and **RAW / JPEG**.

## Controls

Configure viewer mouse and key actions in **Settings › Controls** and **Settings › Preview**:

- **Mouse wheel**: Steps between assets by default over images (**Settings › Controls › Mouse wheel in the viewer** can switch it to **Zoom** or **Pan**). Over text, the wheel scrolls normally; ++ctrl++ + mouse wheel always zooms images and text.
- **Right-drag**: Hold the right mouse button and drag up or down to zoom around the pointer (toggle in **Settings › Controls › Right-click drag to zoom**). Right-clicking without dragging opens the context menu.
- **Left-drag and clicks**: Left-drag pans a zoomed image or selects text. Double-click closes the viewer (except over text, where it selects a word). Middle-click opens the shown file with the default app.
- **Space and fullscreen**: ++space++ toggles the viewer (or plays and pauses media); configure its mode in **Settings › Controls › Space**. Press ++f11++ to toggle fullscreen mode.

For the complete list of keyboard shortcuts, mouse actions and media controls, see [Keyboard and mouse reference](../customize/shortcuts.md).

## Closing

Close the viewer and return to the grid at your current selection in any of these ways:

- Press ++escape++ (exits fullscreen first if active, otherwise closes the viewer or separate asset window).
- Click the **Close** button in the viewer toolbar.
- Double-click the stage on non-text assets.
- Press ++alt+left++ or your mouse's **Back** side button (when set to **Back / forward**): **Back** closes an open viewer before stepping back through view history.
- Press ++space++ (when **Space** is set to **Toggle viewer**) or release ++space++ (when set to **Hold to preview**).
