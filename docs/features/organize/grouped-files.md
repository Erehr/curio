# Grouped files

Related files that belong together can live as **one asset** in Curio. Examples include Live Photos with motion clips, RAW + JPEG camera pairs, `.xmp` or `.aae` sidecars and 3D models with material or buffer files. You can also group AI model weights with previews and metadata, or combine several versions by hand.

## Manual grouping

Select two or more assets in the grid, right-click the one you want as primary and choose **Group as one item**:

- The primary asset holds the other selected assets as grouped **members**.
- The primary asset takes on the combined tags, folders, notes, ratings and custom fields that it lacked. Each member keeps its original file on disk and remembers its prior metadata so splitting later restores it.
- Grouped assets show a stack badge in the grid and list all members in the Inspector **Files** widget.

## Stepping

When you open a manually grouped asset in the viewer:

- Use the viewer's file switcher (or the Inspector **Files** widget) to step between the primary file and any manually grouped member without leaving the asset.
- Each member opens in the viewer for its own file type, such as stepping from a finished PNG to a layered PSD or reference sketch.
- Companion files grouped by automatic rules (such as `.xmp` sidecars or `.mtl` materials) stay attached in the background instead of appearing in the viewer carousel. Viewers use them only where relevant, such as Live Photo playback or RAW + JPEG switching.

## Built-in rules

Curio includes built-in grouping rules in every library:

| Rule | Primaries → Companions | Behavior |
|---|---|---|
| **Live Photo** | `.heic`, `.jpg`, `.jpeg` → `{stem}.mov` (verified by matching Live Photo metadata) | Plays motion video on hover and in the viewer while keeping photo and video together on disk. |
| **Apple edits** | Photos → `{stem}.aae` | Travels with the photo so non-destructive Apple Photos edit sidecars stay paired. |
| **XMP sidecars** | Images and RAW → `{stem}.xmp`, `{name}.xmp` | Keeps Lightroom and Adobe Bridge metadata sidecars attached to the asset. |
| **RAW + JPEG** | Per library: **Off**, **Show JPEG** or **Show RAW** | Pairs matching camera RAW + JPEG files in the same directory so they appear once in the grid and toggle in the viewer. |

## Plugin rules

Plugins can declare companion rules using filename patterns based on the primary file's name (see [Companion file rules](../../plugins/contributions/companions.md)):

- **3D models plugin**: Pairs `.obj` with `{stem}.mtl` (materials) and `.gltf` with `{stem}.bin` (buffers).
- **AI generation plugin**: Pairs `.safetensors`, `.ckpt` and `.pt` model weights with `{stem}.preview.png`, `{stem}.png` or `{stem}.jpg` as the cover thumbnail. Also pairs them with `{stem}.civitai.info` or `{stem}.json` metadata files.
- Third-party plugins can register their own companion rules and optionally mark a companion image as the asset cover.

## Library rules

Open **Settings › Grouped files** (scoped to the active library):

- Switch **RAW + JPEG** between **Off**, **Show JPEG** and **Show RAW**.
- Review every active rule (built-in rules, **RAW + JPEG**, enabled plugin rules and **By hand**), including the primary extensions, companion patterns and grouped file count for each rule.
- Disable a plugin in **Settings › Plugins** to stop its companion rules from running on new imports.

## Survey & apply

Changing **RAW + JPEG** or enabling a plugin does not change existing assets until you run grouping:

1. In **Settings › Grouped files**, click **Check** to preview how many ungrouped files each rule would pair (`+N` badges).
2. Click **Group existing** (or **Group N files**) to apply active rules across the library. Long runs continue in the background.
3. Click **Split all** beside a rule to separate its grouped files into standalone assets without affecting other rules.

## Split & detach

Use the Inspector **Files** widget or the asset context menu to separate grouped files:

- **Detach** (**Files** widget): Removes one member from a grouped asset and turns it back into an independent asset.
- **Split** (**Files** widget or context menu): Separates an asset into all of its individual files and restores their prior metadata. Assets split by hand are excluded from automatic rules so future scans do not regroup them.
- **Allow grouping** (context menu): Lets automatic companion rules pair a previously split asset again.
- **Forget** (**Files** widget): Removes a missing or offline member from the asset and library without deleting the primary asset. Future scans ignore it unless the file changes.

For automation and scripting, see the [Item grouping API](../../api/operations/item-grouping.md).

## File operations

Every asset operation keeps grouped files synchronized:

- **Rename**: Renaming a grouped asset renames the primary file and updates matching companion filenames (`{stem}.*`) where disk access allows.
- **Move and folders**: Moving a managed grouped asset between folders moves its primary and companion hard links together in `<library>.curio/files/`.
- **Trash and restore**: Trashing a grouped asset moves the primary and all members to **Trash** together. Restoring brings the whole group back.
- **Export and transfer**: Exporting files or copying and moving assets between libraries brings grouped companions along so models, Live Photos and sidecars arrive intact.
