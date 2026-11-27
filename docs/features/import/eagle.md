# Eagle

Import **Eagle** `.library` folders into Curio while keeping your folder hierarchy, tags, tag groups, ratings, notes, source URLs, colors and supported smart folders. Copy files into Curio's managed storage or index the Eagle library in place.

## Source & target

Start an Eagle import in any of four ways:

1. Click the **+** (**Add**) button in the sidebar header and choose **Import Eagle library** or **Add Eagle library as location**.
2. Open **Settings › Storage › Import** and click **Import Eagle library** or **Add Eagle library as location**.
3. Drop a `.library` folder onto the Curio window to copy it in, or onto a folder to nest it.
4. Programmatically, pass a `.library` path to [`location.add`](../../api/operations/location.md#location-add) to index it in place.

Curio checks that the chosen folder contains Eagle's `metadata.json` and `images/` layout before starting. Menu and Settings imports place the Eagle folder tree at the top level of your Curio library.

## Storage mode

| Mode | What happens on disk | When to choose it |
|---|---|---|
| **Import Eagle library** (managed storage) | Copies (or hard-links on the same volume) every asset from `<Eagle>.library/images/` into `<Curio>.curio/files/`, organized into real folders with real filenames instead of Eagle's `<id>.info` directories. | Best when moving permanently off Eagle so your files live in a readable folder tree. |
| **Add Eagle library as location** (in place) | Adds the Eagle library's `images/` directory as an in-place location without duplicating file bytes on disk. | Best for trying Curio alongside Eagle or avoiding extra disk space on large libraries. |

!!! tip "Re-importing the same Eagle library"
    Curio tracks which Eagle folders, smart folders, tag groups and assets it has already imported. Running another import from the same `.library` skips previously imported assets and reuses the folders, smart folders and tag groups created on the first pass.

## Metadata

Curio maps Eagle's metadata during import:

- **Folders & hierarchy**: Recreates nested Eagle folders, descriptions, colors and pinned folders (in Curio's **Pinned** section). Eagle's named colors (`red`, `orange`, `yellow`, `green`, `aqua`, `blue`, `purple` and `pink`) map to matching Curio swatches, and hex colors are kept.
- **Auto-tags**: Eagle folder auto-tags attach to their Curio folders after existing assets are imported, matching Eagle's rule that auto-tags apply only to newly added assets.
- **Tags & tag groups**: Imports all tags, starred tags and tag groups.
- **Asset metadata**: Preserves display name, notes, star rating (`0`–`5`), source URL, timestamps and animation or dimension metadata (including WebP animation checks).

## Smart folders

Curio converts Eagle smart folder rules into Curio's [Query language](../../api/query/syntax.md) wherever an equivalent exists. Supported rules cover tags, folders, file extensions, ratings, colors, dimensions, dates, notes, URLs, shapes and boolean combinations:

- Converted smart folders appear under **Smart Folders** with their original names and nesting.
- Curio skips any Eagle smart folder that uses a condition with no Curio equivalent and reports the skipped count in the import summary. You can rebuild those with [Build smart folders](../search/smart-folders.md).

## Limitations

- **Folder name collisions**: Because Curio mirrors managed folders as real directories in `files/`, sibling Eagle folders with the same name cannot share one directory name. Curio appends a numeric suffix to the colliding sibling while keeping all asset memberships.
- **Unsupported Eagle-only extras**: Features specific to Eagle's `.info` format (such as region comments or Eagle plugin data) are not imported. Curio never modifies or deletes your original `.library` folder.
- **Verifying the result**: When import finishes, Curio reports counts of imported assets, folders, smart folders, skipped smart folders, unreadable files and already-present assets. Open **All**, spot-check a few folders and smart folders, and let background thumbnailing and color palette extraction finish.
