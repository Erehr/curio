# Layout

Curio organizes its window in four levels: **window orientation**, **screen**, **zone** and **widget**. Each orientation (Landscape or Portrait) holds per-screen layouts of zones (**Toolbar**, **Filter bar**, **Library**, **Inspector**) filled with draggable widgets. Open **Layout button › Edit layout** or **Main menu › Edit layout** to rearrange controls.

## Screens

- **Landscape and Portrait layouts**: Curio stores one layout for landscape windows and one for portrait windows, switching automatically when you resize past a square aspect ratio. Choose from built-in presets (**Default**, **Vertical**, **Focus**, **Minimal**, **Compact**, **Merged**, **Split**) or duplicate a layout in **Settings › Layout**.
- **Per-screen layouts**: Switch **Screen** in **Edit layout** to tailor **Main**, **Folder**, **Tags**, **Image**, **Video**, **Other files** and plugin viewers (such as **Fonts**, **Reader** or **3D models**). The **Library** sidebar is shared across screens so edits stay consistent across views.
- **Per-library layouts**: Enable **Settings › Layout › Per-library layout** to save layouts, view mode, sort and pinned filters in each library's `layout.json` file instead of sharing one layout across libraries.

## Docking

In **Settings › Layout**, click **Edit** on a layout to set its panel positions:

- Dock the **Library** sidebar on the left or right.
- Dock the **Inspector** on the right, on the left, **Under the grid**, as a **Library tab** or **Below the Library** with a divider. Under the grid, choose single column, grid, masonry or single row.
- Show or hide the window's minimize, maximize and close buttons in **Settings › General › Window buttons** when you want extra toolbar width.

## Panel states

Cycle the **Library** sidebar with ++ctrl+b++, cycle the **Inspector** with ++ctrl+alt+b++ or toggle both with ++ctrl+shift+b++:

| State | Behavior |
|---|---|
| **Full** | Shows the full panel at the width you dragged it to. |
| **Compact** | Shows a slim icon rail with tooltips. Hovering or clicking **Folders**, **Smart Folders**, **Tags** or **Locations** opens flyout menus that support drag and drop. In compact Inspector mode, each widget opens as a flyout from its icon. |
| **Hidden** | Hides the panel until you recall it by shortcut, toolbar button or edge hover. |
| **Edge hover** | Reveals a hidden sidebar when the pointer rests against the window edge (toggle in **Settings › Layout**). |

## Widgets

In **Edit layout** mode, placed widgets show drag handles and remove (`×`) buttons while the palette lists all available widgets:

- **Add**: Drag any widget from the palette into the **Toolbar**, **Filter bar**, **Library** or **Inspector**. You can place the same widget more than once.
- **Move or reorder**: Drag placed widgets within a zone or across zones.
- **Remove**: Click a widget's `×` button or drag it back onto the **Edit layout** window.
- **Adaptive rendering**:
  - **Inline widgets** (Search, Filters, Breadcrumb, Back and forward, Thumbnail size, Zoom) stay inline inside bars.
  - **Panel widgets** (Preview, Rating, Tags, Folders, Properties, Summary, Spectrum map and plugin widgets) span the width of sidebars and collapse to popup icons inside toolbars or compact rails.
  - **Buttons** (Menu, Add, Switch library, Actions and plugin windows) stay compact buttons in every zone.

## Spacers & pins

Use layout parts from the **Edit layout** palette to align controls and pin sidebar sections:

- **Row**: Groups multiple icon buttons on one horizontal line inside a vertical sidebar.
- **Gap**: Adds fixed spacing equal to one row or icon width.
- **Elastic gap**: Expands to fill remaining space, pushing later widgets to the right end of a toolbar or the bottom of a sidebar.
- **Separator**: Draws a thin divider line (vertical in toolbars and rows, horizontal in sidebars). Separators appear only where you place them.
- **Pinned top & bottom**: Move the sidebar pin dividers to keep top or bottom widgets fixed while the middle section scrolls.

## Plugin widgets

- **Custom fields & plugins**: Every custom field, plugin sidebar or Inspector widget, user action and plugin window button appears in the **Edit layout** palette.
- **Viewer toolbars**: Switch to a viewer screen (such as **Image**, **Fonts**, **Reader** or **3D models**) in **Edit layout** to rearrange built-in viewer controls alongside plugin toolbar controls.

## Reset

- Click **Reset** in **Edit layout** to restore the active screen and orientation to defaults.
- Click **Done** or press ++escape++ to exit layout editing.
