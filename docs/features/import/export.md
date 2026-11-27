# Export

Export asset files for other programs or save your tags, folders, notes, ratings and custom fields in **JSON**, **CSV** or per-file `.curio.json` sidecars.

## Files

1. Select one or more assets in the grid.
2. Right-click and choose **Export › Files** (++ctrl+shift+s++), or click **Export › Export files** in the Inspector.
3. Pick a destination folder on disk.

Curio copies the selected files into that destination without modifying your library or originals. Missing or offline assets are skipped and reported. To automate exports, call [`item.export`](../../api/operations/item-lifecycle.md#item-export) through the API.

## JSON & CSV

For selected assets, right-click and open **Export** (**Metadata as JSON** or **Metadata as CSV**). To export the current selection or view, use **Main menu** (**Export metadata (JSON)** or **Export metadata (CSV)**):

=== "JSON (`curio-metadata/1`)"

    Writes a single UTF-8 JSON file with your custom field definitions alongside every exported asset:

    ```json
    {
      "format": "curio-metadata/1",
      "exported_at": 1763940000000,
      "fields": [ /* FieldDef objects */ ],
      "assets": [
        {
          "uid": "a1b2c3d4-...",
          "path": "C:\\Users\\You\\Pictures\\hero.png",
          "name": "hero.png",
          "display_name": "Hero concept",
          "ext": "png",
          "mime": "image/png",
          "kind": "image",
          "size": 2481920,
          "width": 2560,
          "height": 1440,
          "duration": null,
          "created": 1763000000000,
          "modified": 1763100000000,
          "imported": 1763200000000,
          "rating": 5,
          "favorite": true,
          "explicit": false,
          "annotation": "Final key art",
          "source_url": "https://example.com/art/42",
          "tags": ["concept", "key-art"],
          "folders": ["Characters"],
          "palette": ["#2b3a4a", "#e6b87d"],
          "quick_hash": "...",
          "content_hash": "...",
          "status": "Available",
          "fields": { "artist": "M. Chen" },
          "overrides": ["artist"]
        }
      ]
    }
    ```

=== "CSV"

    Writes an RFC 4180 comma-separated file suited for spreadsheets and scripts. Multi-value lists (`tags`, `folders` and multiple-choice custom fields) are joined with `; `.

    **Columns**: `uid`, `path`, `name` (uses the display name if set, otherwise the filename), `ext`, `size`, `width`, `height`, `rating`, `favorite`, `explicit`, `tags`, `folders`, `annotation`, `source_url`, followed by one column per custom field key in the library.

## Sidecars

Curio can write one `.curio.json` sidecar file per asset using the `curio-metadata/1` format:

- **Sidecars to folder** (**Main menu › Export sidecars** or **Export › Sidecars to folder**): Writes `<uid>-<sanitized_name>.curio.json` for every exported asset inside the folder you choose. Works for both managed and location assets.
- **Sidecars beside files** (**Export › Sidecars beside files**): Writes `<original_path>.curio.json` next to each original file on disk.
  - Allowed only for assets in indexed locations set to **Edit files** or **Edit and organize**. Managed assets inside `<Library>.curio/files/` and assets in **Read-only** locations are skipped.
  - **Safety guard**: If a `.curio.json` file already exists at the target path, Curio overwrites it only when its `format` field is `curio-metadata/1` (confirming Curio wrote it).

## Grouped files

- When exporting files from a grouped asset, Curio exports the primary file along with its grouped companion files and resolves filename collisions in the destination directory.
- In JSON and sidecar exports, grouped assets include a `"parts"` array listing each member's `name`, `role`, `ext` and `size`.

## Included fields

| Metadata | File export | JSON & Sidecar (`curio-metadata/1`) | CSV |
|---|---|---|---|
| Original file bytes | Yes | No (path recorded) | No (path recorded) |
| Stable UID, filename, display name, extension, size, dimensions | Filename only | Yes | Yes (`name` uses display name if set, else filename) |
| Rating, favorite, explicit, notes (`annotation`), source URL, tags, folder names | No | Yes | Yes |
| Custom field values | No | Yes (typed JSON) | Yes (stringified; `; ` for lists) |
| Custom field definitions & manual override list (`overrides`) | No | Yes | No |
| Color palette, MIME type, timestamps, hashes, grouped `parts` | No | Yes | No |

## Backups

Exports provide **portability, spreadsheets and archival interoperability**, not full library restoration. Use [Back up and restore](../libraries/backup.md) to preserve your complete library database. Backups keep folder hierarchy with icons, colors and auto-tags, smart folders, tag groups, library settings and plugin asset state.
