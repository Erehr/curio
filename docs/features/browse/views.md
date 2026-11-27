# Views

Switch views and configure captions, columns, sorting and grouping from the **Layout** popover in the top toolbar. You can apply view settings globally or override them per folder.

## Layouts

Curio includes five layouts for browsing results:

| View | Layout geometry | Best for |
|---|---|---|
| **Grid** | Uniform fixed-ratio cards with optional captions below each thumbnail. Thumbnails fill their tiles. | Mixed media, folders and icon sets. |
| **Masonry** | Vertical columns that preserve each asset's aspect ratio, including custom covers and book or comic covers. | Mixed portrait and landscape art, photos, posters and ebooks. |
| **Justified** | Horizontal rows scaled so each row fills the full width while keeping each asset's aspect ratio. | Photo galleries and reference boards. |
| **List** | Virtualized table rows with sortable, resizable and reorderable columns, including custom fields and font previews. | Comparing dimensions, file sizes, dates, extensions and metadata. |
| **Spectrum** | 2D radial color map arranging assets by dominant hue and lightness. | Color exploration. See [Explore colors with Spectrum](spectrum.md). |

Drag the **Size** slider in the **Layout** popover, or hold ++ctrl++ and scroll the mouse wheel over the grid, to resize thumbnail cards and subfolder tiles.

## Captions & cards

Configure card details in the **Layout** popover for Grid, Masonry and Justified views (caption toggles do not appear in Spectrum; List view keeps only **Show extension**, which ends each row's name):

- **Show name**, **Show extension**, **Show dimensions**, **Show note**: Choose which caption lines appear below cards. Pixel dimensions appear only for image and video assets; books, PDFs and archives do not fall back to file size.
- **Show format badges**: Shows compact format, duration, animation and grouped-file badges on thumbnails.
- **Explicit blur**: Assets flagged as **Explicit** show a 10 px blur over thumbnails across Grid, Masonry, Justified, List and Spectrum views. They remain unblurred in the Inspector preview and full viewer.
- **Show subfolders** and **Show subfolder contents**: Independent toggles in the **Layout** popover. When **Show subfolders** is on, immediate subfolders appear above the grid as 4:3 cover-stack cards (or rows in List view) scaled by **Size**. Single-click selects a subfolder card; double-click or ++enter++ opens it.

## List columns

In **List** view, right-click the table header or open the column picker to show or hide columns:

- **Built-in columns**: Name, Type, Extension, File size, Dimensions (images and videos), Duration, Rating, Tags, Folders, Date added, Date modified, Date created, Path, Note and URL.
- **Resize and reorder**: Drag column dividers to resize columns and drag column headers to reorder them. Click any sortable column header to toggle ascending or descending sort.
- **Fit**: Right-click a header and choose **Fit column** or **Fit all columns** to size columns to their content, or double-click a column's divider. **Auto-fit columns** keeps them fitted as results change, and when the list gets narrower the columns share its width instead of scrolling sideways, down to a minimum width each.

## Custom columns

- **Custom field columns**: Toggle any indexed custom or plugin field on as a List column, such as Camera, Lens, Author, Series, Prompt, Model or Font category.
- **Plugin columns**: Plugins can contribute specialized List columns. For example, the **Font sample** column renders live sample text in each font's typeface and syncs with the window's sample text.

## Sorting

Open **Sort by** in the **Layout** popover or click a List column header to sort ascending or descending:

- **Standard properties**: Date added, Date modified, Date created, Name, Extension, File size, Rating, Dimensions, Duration and Random. Folders carrying a saved manual order (such as imported Eagle folders) also show **Manual** while that order is active.
- **Custom fields**: Any sortable indexed custom or plugin field. Sorting by a primary field with a paired secondary field (such as **Series** and **Number in series**) breaks ties ascending by the secondary field. Paired secondary fields do not appear separately in **Sort by** or **Group by**.

## Grouping

Curio supports **Group by** across Grid, Masonry, Justified and List views (not Spectrum):

- **Group criteria**: **None**, **Type**, **Extension**, **Date added**, **Date modified**, **Date created**, **Rating**, **File size**, **Name**, **Location** or any indexed single-value custom field.
- **Direction and collapsing**: Set ascending or descending group order independently. Click a group header to collapse or expand that section; collapsed sections skip layout and thumbnail loading.
- **Section context menu**: Right-click any group header to choose **Select items in group**, **Collapse all groups** or **Expand all groups**.
- **Per-folder overrides**: Set custom grouping and layout options per folder while keeping subfolder cards pinned above grouped assets.

## Custom types

Alongside built-in asset types (**Images**, **Videos**, **Audio**, **Documents**, **Code**, **Links**, **Archives**, **Fonts**, **3D models** and **Other**), format plugins can register custom display types. For example, the **Reader** plugin registers **Books** and the **AI generation** plugin registers **Models**. The **3D models** plugin uses the built-in **3D models** type. Active custom types appear in **Group by › Type**, the **Types** filter chip and the List **Type** column. See [Display contributions](../../plugins/contributions/display.md).
