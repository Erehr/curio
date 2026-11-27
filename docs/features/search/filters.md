# Filters

The **Filter bar** (++ctrl+shift+l++) sits above your results with popovers, live counts and reusable filter presets. Clicking an open filter chip closes it again.

## Color

Click the **Color** chip to filter by dominant palette color:

- Pick a color from the hue and saturation selector, click a swatch or enter a hex value (`#3b9160`).
- Drag the **Accuracy** slider to widen or tighten perceptual color matching.
- Select **Grayscale** to match monochrome, black-and-white and neutral-toned assets.

## Tags & folders

- **Tags**: Search tags by name, browse groups, starred or recent tags and choose **Any of these tags**, **All of these tags** or **Exactly these tags**. Right-click a tag to exclude it, or select **Untagged** to match assets without tags.
- **Folder** and **Folder color**: Include or exclude library folders, or filter by **Folder color** to match assets inside folders with a given color swatch.

## Shape & types

- **Types**: Filter by built-in types (**Image**, **Video**, **Audio**, **Document**, **Code**, **Link**, **Archive**, **Font**, **3D model**, **Other**), custom plugin types (such as **Books** or **Models**) and **Animated** (animated GIF, animated WebP and APNG).
- **Shape**: Filter by aspect category (**Landscape**, **Portrait**, **Square**, **Wide panorama**, **Tall panorama**, **4:3**, **16:9** or **Custom** ratio). Selected shapes appear directly on the chip label.
- **Dimensions**: Set minimum and maximum pixel width and height for image and video assets.

## Properties

- **Extension**: Pick one or more file extensions (`png`, `jpg`, `mp4`, `epub`, `safetensors`) with live asset counts.
- **Rating**: Select any combination of star ratings (**Unrated** through **5 stars**). Use the two tristate checkboxes below the divider to include or exclude **Favorite** and **Explicit** assets.
- **Size**, **Duration** and **Pages**: Set minimum and maximum numeric bounds for file size, media duration or page count.

## Dates & text

- **Date added**, **Date modified** and **Date created**: Filter with relative ranges (Today, Last 7 days, Last 30 days, Last 3 months, Last year) or explicit start and end dates.
- **Note** and **URL**: Filter for assets that have or lack notes or URLs, or match specific keywords and domains.

## Custom & plugins

Indexed custom fields and plugin contributions integrate directly into the **Filter bar**:

- **Dedicated filter chips**: Fields configured to appear with the main filters (such as **Model** and **LoRA** from the **AI generation** plugin) show as standalone chips. Other indexed fields appear under **More filters**.
- **Multi-value lists and paired ranges**: Custom list filters include a search box above their values. Paired numeric ranges for multi-value fields (such as **LoRA strength** for **LoRA**) appear inside the parent popover instead of **More filters**. Single-value paired fields (such as **Number in series** for **Series**) stay available both standalone and inside their parent filter.
- **Grouped numeric filters**: Related numeric fields (such as Width, Depth and Height in the **3D models** plugin) appear together in one multi-range popover (**Model size (mm)**).
- **Plugin presets**: Plugins can register named one-click filters (such as *Textured models* or *≤ 12 steps*) in the **Presets** menu.

## Facet counts

Every filter popover calculates live counts scoped to your current view and other active filters. You see how many assets match each tag, extension, rating, type or field value before clicking. When multiple values are selected, they appear separated by commas on the active chip.

## Presets

- **Pin chips**: Pin frequent filter chips (including custom fields) so they stay visible on the **Filter bar** while less used filters remain under **More filters**.
- **Lock filter**: Click **Lock filter** to keep active filters applied as you switch between folders, locations and smart folders. Click **Unlock filter** to release them.
- **Clear filters**: Click the clear icon on a chip (or middle-click the chip) to remove one filter, or click **Clear filters** to reset the bar.
- **Saved filters**: Open **Saved filters** and choose **Save current filters** to store a reusable preset, or choose **Save as smart folder** to create a [Smart folder](smart-folders.md).
