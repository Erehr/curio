# Companions

Companion rules (`formats[].companions[]`) collapse related files in the same directory into **one item** (for example `.obj` + `.mtl`, `.gltf` + `.bin` or `.safetensors` + preview cover + `.civitai.info`).

See [Format and companion declarations](../manifest/formats.md#formats-companions) for every key, type and default.

## Rules

```json
{
  "formats": [
    {
      "exts": ["obj"],
      "kind": "model",
      "viewer": "index.html",
      "companions": [
        { "role": "materials", "files": ["{stem}.mtl"] }
      ]
    }
  ]
}
```

- **Rule ID**: Each companion declaration produces a stable rule ID **`<plugin_id>/<role>`**, shown in **Settings › Grouped files** and [`library.grouping`](../../api/operations/library-grouping.md#library-grouping).
- **Role queries**: `role` accepts 1 to 40 ASCII characters (`a-z`, `A-Z`, `0-9`, `-`, `_`) and is queryable via `part = <role>` (for example `part = materials` or `part = preview`).

## Templates

Every entry in `files[]` must start with `{stem}` or `{name}`, followed by literal suffix characters (no path separators `/`, `\`, wildcards or control characters):

| Template | For primary `hero.safetensors` | Matches |
|---|---|---|
| `{stem}.preview.png` | Stem `hero` + `.preview.png` | `hero.preview.png` |
| `{stem}.mtl` | Stem `hero` + `.mtl` | `hero.mtl` |
| `{name}.xmp` | Full name `hero.safetensors` + `.xmp` | `hero.safetensors.xmp` |

!!! note "No standalone exact filenames"
    Exact-filename templates without `{stem}` or `{name}` (such as `"cover.jpg"`) are disallowed to prevent filename collisions across items sharing a managed folder in `<Library>.curio/files/`.

## Cover

Set `"cover": true` on a companion rule (such as AI model previews) to generate the primary item's thumbnail from the first matching companion file in `files[]` order:

```json
{
  "role": "preview",
  "cover": true,
  "files": ["{stem}.webp", "{stem}.preview.webp", "{stem}.preview.png", "{stem}.png", "{stem}.jpg"]
}
```

## Verification

Built-in rules can specify `"verify": "live-photo"`, which verifies Apple Live Photo content identifiers before pairing a `.heic` or `.jpg` with `{stem}.mov` so unrelated videos sharing a filename stem are not grouped.

## Precedence

- Curio evaluates rules in deterministic priority order across the files of one directory.
- Items split manually by the user (`item.split`) stay ungrouped until the user calls `item.allowGrouping`.
- Enabling a plugin applies its companion rules to new imports and rescans. Users can preview and apply new rules to existing library items in **Settings › Grouped files** (**Check existing files** → **Group existing files**).

## Accessing parts

- **Declarative JSON extractors**: Set `extractors[].json.role` to read fields from a grouped companion JSON file (`{stem}.civitai.info`, `{stem}.metadata.json`) using JSON Pointers.
- **Page extractors**: `plugin.extract` receives a `parts[]` array with `{ role, name, item, size, fileUrl }` for every grouped member.
- **Command extractors**: The `{parts}` placeholder passes a JSON array of `{ role, path }`.
- **Viewer pages**: `item.fileUrl` resolves sibling files and grouped members relative to the primary file URL.
- **Exports & transfers**: Exporting (`item.export` with default `members: true`), renaming, moving folders, trashing and copying across libraries keep companion members attached.

## Patterns

| Use case | Primary `exts` | Companion rule |
|---|---|---|
| Wavefront OBJ materials | `["obj"]` | `{ "role": "materials", "files": ["{stem}.mtl"] }` |
| glTF binary buffers | `["gltf"]` | `{ "role": "buffers", "files": ["{stem}.bin"] }` |
| AI model cover previews | `["safetensors", "sft", "ckpt", "gguf"]` | `{ "role": "preview", "cover": true, "files": ["{stem}.webp", "{stem}.preview.png", "{stem}.png", "{stem}.jpg"] }` |
| AI model sidecar JSON | `["safetensors", "sft", "ckpt", "gguf"]` | `{ "role": "metadata", "files": ["{stem}.metadata.json", "{stem}.civitai.info", "{stem}.json"] }` |
