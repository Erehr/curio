# Shortcuts

Inspect, rebind or clear every shortcut in Curio under **Settings › Shortcuts**, including viewer controls, user actions and plugin shortcuts.

On macOS, `Ctrl` shortcuts map to `Cmd` (`⌘`) and `Alt` shortcuts map to `Option` (`⌥`). **Move to Trash** defaults to `Cmd+Backspace` (`⌘⌫`) on macOS.

## Main window

| Action | Default shortcut | Action | Default shortcut |
|---|---|---|---|
| Cycle Library sidebar | ++ctrl+b++ | Search | ++ctrl+f++ |
| Cycle Inspector | ++ctrl+alt+b++ | Toggle Filter bar | ++ctrl+shift+l++ |
| Toggle both panels | ++ctrl+shift+b++ | Clear filters | ++ctrl+shift+backspace++ |
| Filter Library sidebar | ++ctrl+alt+f++ | Settings | ++ctrl+comma++ |
| All / Uncategorized / Untagged | ++ctrl+1++ / ++ctrl+2++ / ++ctrl+3++ | Switch library | ++ctrl+shift+o++ |
| Recently used / Random | ++ctrl+4++ / ++ctrl+5++ | Back / Forward | ++alt+left++ / ++alt+right++ |
| Tag manager / Trash | ++ctrl+6++ / ++ctrl+7++ | Preview | ++space++ |
| Grid / Justified / Masonry / List / Spectrum | ++ctrl+shift+1++ … ++ctrl+shift+5++ | Open with default app | ++shift+alt+o++ |
| Zoom thumbnails or viewer in / out | ++ctrl+equal++ / ++ctrl+minus++ | Show in file manager | ++shift+alt+r++ |
| Scroll to top / bottom | ++home++ / ++end++ | Open in new window | ++shift+alt+w++ |
| Page up / down | ++page-up++ / ++page-down++ | Rename / Batch rename | ++f2++ |
| New folder / New subfolder | ++ctrl+shift+n++ / ++shift+alt+n++ | Copy files / Copy file path | ++ctrl+c++ / ++shift+alt+c++ |
| Expand / collapse all folders | ++slash++ | Copy tags / Paste tags | ++ctrl+shift+c++ / ++ctrl+shift+v++ |
| Export files | ++ctrl+shift+s++ | Paste files, image, link or path | ++ctrl+v++ |
| Duplicate | ++ctrl+d++ | Select all | ++ctrl+a++ |
| Delete / Move to Trash | ++delete++ (macOS: ++cmd+backspace++) | Add tags / Tags picker | ++ctrl+k++ / ++t++ |
| Add to folders / Folders picker | ++ctrl+shift+d++ / ++f++ | Add to last folder | ++alt+d++ |
| Rate 0–5 stars | ++0++ … ++5++ | Actions menu | ++g++ |
| Toggle favorite / Toggle explicit | *(Unbound; bind in Settings)* | Black and white | ++alt+g++ |
| Thumbnail from file | ++ctrl+shift+t++ | Quit Curio | ++ctrl+q++ |

## Viewer

| Input | Action |
|---|---|
| ++left++ / ++right++ (or ++up++ / ++down++) | Previous or next asset in results |
| ++page-up++ / ++page-down++ | Previous or next file inside a manually grouped asset |
| ++escape++ / double-click / **Back** | Close viewer and return to the grid |
| ++ctrl+equal++ / ++ctrl+minus++ / ++ctrl+0++ | Zoom in, zoom out or fit / actual size (text viewer: reset to 100%) |
| ++r++ / ++h++ | Turn view 90° / Mirror horizontally |
| ++m++ / ++alt+z++ / ++l++ | Text viewer: cycle Markdown view (Rendered, Source, Side by side), wrap lines or toggle line numbers |

## Media

| Input | Action |
|---|---|
| ++space++ | Play or pause |
| ++ctrl+up++ / ++ctrl+down++ | Volume ±10% |
| ++ctrl+right++ / ++ctrl+left++ | Skip forward or back (1/10 of length, up to 5 seconds) |
| ++]++ / ++[++ | Faster or slower playback speed (0.25×–5× video, 0.5×–2× audio) |
| ++comma++ / ++period++ | Step 1 frame back or forward (`Alt+,` / `Alt+.` for 10 frames) |
| ++alt+c++ / ++alt+s++ | Copy current video frame / Save frame as image |

## Specialized viewers

Plugin and specialized viewer shortcuts work only while that viewer is open and take priority over global grid shortcuts:

- **Fonts viewer**: ++s++ (Sample text), ++w++ (Waterfall), ++y++ (Glyphs), ++i++ (Details).
- **Reader viewer**: ++t++ (Sidebar), ++b++ (Bookmark), ++[++ / ++]++ (Previous / next bookmark), ++shift+[++ / ++shift+]++ (Previous / next chapter), ++slash++ (Search), ++d++ (Double page), ++c++ (Continuous), ++m++ (Right to left), ++w++ (Fit), ++g++ (Page grid), ++s++ (Grid style), ++k++ (Next theme), ++equal++ / ++minus++ / ++0++ (Text size / zoom).
- **3D models viewer**: ++f++ (Fit), ++g++ (Floor grid), ++e++ (Edges), ++t++ (Textures), ++i++ (Details), ++s++ (Next shading).

## Tools

- **Duplicates window**: ++up++ / ++down++ moves between groups, ++space++ toggles group inclusion, ++1++–++9++ keeps a member, ++ctrl+enter++ scans or moves copies to Trash and ++escape++ closes the window.
- **Filter bar checklists**: Left-click, ++space++ or ++enter++ includes a value. Right-click or ++shift+enter++ excludes it. Middle-click any active filter chip to clear it.
- **Inspector chips**: Middle-click a tag or folder chip, or click its hover `×` button, to remove it.
- **Dialogs**: Global window shortcuts pause while a dialog is open. Press ++escape++ to close the dialog.

## Mouse

Configure pointer and wheel behavior in **Settings › Controls** and **Settings › Preview**:

| Input | Configurable behavior |
|---|---|
| **Mouse wheel in the viewer** | **Previous / next item** *(default)*, **Zoom** or **Pan**. ++ctrl+wheel++ always zooms; in Spectrum, the wheel zooms about the pointer without Ctrl. |
| **Mouse wheel (video & audio)** | **Seek** (5 s), **Volume** or **Previous / next item** (**Settings › Preview › Video & audio playback**). |
| **Mouse side buttons** | **Back / forward** *(default)*, **Previous / next item** or **Nothing** (**Settings › Controls**). |
| **Right-click drag to zoom** | Drag up or down in the viewer to zoom (toggle in **Settings › Controls**). |
| **Middle click** | **New window** *(default)*, **Open with default app** or **Nothing**. It acts when the button is released without moving: hold and drag in the grid or list to scroll, faster the further you drag (in Spectrum, the drag pans). |
| **Space** | **Toggle viewer** *(default)*, **Hold to preview**, **Play / pause** or **Scroll**. |

## Rebind

In **Settings › Shortcuts**:

1. Click a shortcut cell and press your key combination. Press ++escape++ to cancel recording.
2. Click the clear (`×`) button beside a shortcut to leave it unassigned.
3. Click **Reset all shortcuts** to restore defaults.

Each global shortcut belongs to one action. Assigning a key already in use transfers it to the new action and names the displaced action. Any remaining conflict is marked in the list. You can bind ++backspace++ and ++delete++ like any other key.

## Actions & plugins

- **Your actions**: Every reusable action from [Build reusable actions](../organize/actions.md) appears in its own group in **Settings › Shortcuts**.
- **Plugin actions & viewer keys**: Each enabled plugin's actions and viewer shortcuts appear in dedicated groups. If a plugin default collides with an existing shortcut, it stays unassigned until you bind it. Plugin tooltips and pages update automatically to display your custom bindings.
