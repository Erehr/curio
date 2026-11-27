# Tags

Tags describe assets across folder boundaries: subjects, styles, statuses, clients or anything else that cuts across your hierarchy. Tag groups organize tags into named collections with icons, descriptions and inherited colors. A single tag can belong to multiple groups at once.

## Assign & copy

Tag one asset or a whole selection from anywhere in the library:

| Action | Shortcut | Where else to find it |
|---|---|---|
| Open the tag picker for the selection | ++t++ or ++ctrl+k++ | Asset context menu › **Add tags**, or Inspector **Tags** widget › **New tag** |
| Copy tags from the selection | ++ctrl+shift+c++ | Asset context menu › **Copy › Tags**, or right-click Inspector **Tags** heading › **Copy tags** |
| Paste copied tags onto the selection | ++ctrl+shift+v++ | Asset context menu › **Paste tags**, or right-click Inspector **Tags** heading › **Paste tags** |
| Remove a single tag | None | Click the remove button on its Inspector chip, or uncheck it in the tag picker |
| Remove all tags from the selection | None | Asset context menu › **Remove all tags** (disabled when none have tags), or right-click Inspector **Tags** heading › **Remove all tags** |
| Drag assets onto a tag | None | Drop selected assets onto any tag in the sidebar **Tags** list |

Copying tags stores the unique tag names for pasting onto other assets in the same window and copies them to the system clipboard as a comma-separated list. Right-click any tag chip in the Inspector to **Rename tag**, **Copy tag** or **Remove** it from the selection.

## Tag picker

Press ++t++ or ++ctrl+k++ to open the floating tag picker beside the Inspector or selected asset. Changes apply immediately as you check or uncheck rows, and the picker stays open so you can toggle several tags in a row:

- **Search and create**: Type to fuzzy-search tag names and tag group names. Closer matches rank first, with ties broken by asset count. If no tag matches exactly, click **Create "&lt;name&gt;"** or press ++enter++ to create it and attach it to the selection.
- **Sections**: Without a search term, tags appear in collapsible sections: **Starred**, **Recent** (up to 12 per library), each tag group and **Uncategorized** (or **Others**). Right-click any section header to collapse or expand all sections.
- **Header toggles**: Show or hide the left **Tag groups** pane, the **Starred** section and the **Recent** section. Your choices persist across sessions.
- **Star inline**: Right-click any tag row in the assignment picker to star or unstar it. When opened from the **Tags** filter on the filter bar, right-clicking a row excludes that tag instead; see [Use filters and presets](../search/filters.md).
- **Keyboard**: Press ++arrow-up++ and ++arrow-down++ to move through rows, ++enter++ to toggle, ++tab++ to switch focus between the tag list and **Tag groups**, and ++escape++ to close.

## All Tags

Click **All tags** in the Library sidebar or press ++ctrl+6++ to open the Tag Manager. Its left pane filters which tags appear in the main area:

- **All tags**: Every tag in the library, including unused tags with count `0`.
- **Uncategorized**: Tags that belong to no tag group.
- **Starred**: Starred tags that appear at the top of the tag picker with a star icon.
- **Groups**: Every tag group in your custom order, with its icon, color and member tag count.

Double-click any tag in the Tag Manager to filter the grid by that tag, or right-click selected tags and choose **Show items**. Right-click a tag group and choose **Show items** to filter the grid by all tags in that group.

!!! tip "Best rated preview in the Inspector"
    While the Tag Manager is open, the default Inspector shows the **Best rated** widget. It displays up to 12 top-rated assets matching the selected tags, the **Starred** view or the open tag group. Click any thumbnail to jump to all matching assets.

## Manager views

While the Tag Manager is open, the toolbar provides controls for browsing large tag vocabularies:

- **Tags as grid or list**: Switch between a multi-column **Grid** and a single-column **List**.
- **Sort tags**: Open the sort popover to choose:
    - **A > Z** or **Z > A** (case-insensitive alphabetical).
    - **Count (High→Low)** or **Count (Low→High)** (by asset count, ties broken alphabetically).
    - **Group by**: **Alphabet** (grouped under initial letters, or `#` for non-letters; omitted when sorting by count or viewing one group) or **Groups** (grouped under each tag group, plus **Uncategorized**).
- **Search**: Type in the toolbar search box to fuzzy-filter tags by tag name or group name.

Select multiple tags with ++ctrl++-click, ++shift++-click or by dragging a selection box across empty space. Press ++escape++ to clear the tag selection.

## Edit & merge

Select one or more tags in the Tag Manager and right-click, or use keyboard shortcuts:

- **Rename and merge**: Right-click a tag and choose **Rename** (or press ++f2++). Renaming updates the tag everywhere and refreshes full-text search. Renaming to an existing tag name merges them automatically: every asset with the old tag gets the existing tag without duplicates, and the old tag is removed.
- **Star / Unstar**: Right-click one or more tags to star or unstar them, or drag selected tags onto **Starred** in the left pane.
- **Delete**: Right-click selected tags and choose **Delete tag** / **Delete N tags**, or press ++delete++. After confirmation, the tags are removed from all assets while the assets stay untouched.
- **Color**: Tags inherit their color from their first tag group in group order. Individual tag colors are not set in the Tag Manager UI, though the API's [`tag.setColor`](../../api/operations/tag.md) operation can set a fallback color for ungrouped tags.

## Tag groups

Manage tag groups in the Tag Manager's left pane:

- **Create**: Click **+** beside **Groups**, type a name and press ++enter++.
- **Order**: Drag any group up or down in **Groups** to reorder it. Group order controls both the sidebar list and which group's color wins when a tag belongs to multiple groups.
- **Description and inline creation**: Click a group to open it and edit its **Description** beneath the heading (saved when you click away or press ++enter++). Click **+** beside the group title to create a tag inside it.
- **Group context menu**: Right-click a group to **Show items**, **Rename** (++f2++), **Change icon** (from the Tabler set) or pick a **Color** swatch. You can also **Copy to another library** (copies all assets carrying that group's tags) or **Remove** (++delete++). Removing a group deletes only the group; its tags stay in the library.

## Inherited color

A tag can belong to zero, one or multiple tag groups at the same time:

| Action | Effect on existing group memberships |
|---|---|
| Right-click tag(s) › **Add to group** › *group* | Adds the tag to that group while keeping all other group memberships |
| Right-click tag(s) › **Move to group** › *group* | Removes the tag from all other groups and puts it only in the chosen group |
| Drag tag(s) onto a group in the left pane | Moves the tag into that group (replaces previous group memberships) |
| Drag tag(s) onto **Uncategorized** | Removes the tag from all groups |
| Right-click tag(s) › **Remove from group** (inside a group) | Removes the tag from the current group only |
| Right-click tag(s) › **Remove from all groups** (outside a group) | Removes the tag from every group |

Everywhere a tag appears (in the Tag Manager, tag picker, sidebar **Tags** list, Inspector chips and List view **Tags** column), it inherits the color of its **first group** according to your Tag Manager group order. Reordering groups updates inherited colors automatically. For automation and scripting, see the [Tag API](../../api/operations/tag.md) and [Tag group API](../../api/operations/tag-group.md).
