# Extractors

Extractors (`extractors[]`, shown as **metadata readers** in Curio's UI) read searchable values from imported and scanned files into fields declared in `fields[]`. Curio supports five extractor modes: three declarative modes executed natively in Rust, a sandboxed hidden HTML page mode and an external CLI command mode.

See [Extractor declarations](../manifest/extractors.md) for the complete key reference, defaults and required permissions.

## Identity

```json
{
  "extractors": [
    {
      "id": "notes",
      "version": 1,
      "exts": ["txt", "md"],
      "page": "extract.html",
      "fields": ["first_line", "lines"]
    }
  ]
}
```

- **Identifier (`id`)**: Appended to `<plugin_id>/<id>` (defaults to `x<index>` if omitted).
- **Rescan versioning (`version`)**: Increment `version` whenever extraction logic changes so Curio re-reads matching files once per library.
- **Cleared fields (`fields`)**: Declare every field key the extractor populates. Any listed field omitted from a later extraction result is cleared on rescan while manual user overrides remain intact.

## png_text

Copies named PNG text chunks (`tEXt`, `zTXt`, `iTXt`) into fields with no code or permissions:

```json
{
  "id": "png-chunks",
  "exts": ["png"],
  "png_text": {
    "Software": "software",
    "Comment": "prompt"
  }
}
```

## regex

Extracts the first capture group (`(...)`) from a PNG text chunk with a regular expression (no permissions required):

```json
{
  "id": "a1111-params",
  "exts": ["png"],
  "regex": [
    { "chunk": "parameters", "pattern": "Steps: (\\d+)", "field": "steps" },
    { "chunk": "parameters", "pattern": "CFG scale: ([\\d.]+)", "field": "cfg" }
  ]
}
```

## json

Reads values from a grouped JSON companion file using RFC 6901 JSON Pointer syntax (no permissions required):

```json
{
  "id": "civitai",
  "exts": ["safetensors", "ckpt"],
  "json": {
    "role": "metadata",
    "file": ".civitai.info",
    "fields": {
      "base_model": "/baseModel",
      "trigger_words": "/trainedWords",
      "__source_url": "/modelUrl"
    }
  }
}
```

Use `role` to pick the companion role, optional `file` to match a filename suffix and `fields` to map output keys to JSON Pointers.

## Page & command

When binary parsing, archive inspection or custom logic is needed:

- **Page extractor (`page`)**: Requires `files.read`. Runs in a hidden frame while a window is open, receives [`plugin.extract`](../hooks/work.md#plugin-extract) with a Range-capable `fileUrl` and companion `parts[]` and replies via `curio.extractDone(request, values, error)` within 60 s (max 4 MB reply). Requires nothing installed outside the plugin. See [Complete a thumbnail or extraction request](../pages/completions.md).
- **Command extractor (`command`)**: Requires `files.read` and `exec`. Runs once per file (30 s timeout) with `{input}` and optional `{parts}` placeholders, printing a flat `{ "field_key": value }` JSON object to `stdout`. See [Extractor and thumbnail program contracts](../programs/file-handlers.md).

## Special fields

In addition to declared custom fields, any extractor can emit three reserved keys:

- **`__title`**: Fills the item's display name if empty.
- **`__annotation`**: Fills the item's notes if empty.
- **`__source_url`**: Fills the item's source URL if empty.

These special keys never overwrite existing non-empty user values. Curio ignores all other `__*` keys.

## Indexing & errors

- Extractors run on new and changed files only while at least one of their target fields is indexed (`indexed: true` in **Settings › Fields**).
- Unindexed fields can still be inspected on demand without storing values via [`item.readFields`](../../api/operations/item-reads.md#item-readfields), which runs built-in and declarative extractors (not external commands).
- Curio records timeouts, errors or replies over 4 MB in **Settings › Fields** ([`field.failures`](../../api/operations/field.md#field-failures)) and skips retrying until the file changes or the user triggers **Try again** or [`field.rescan`](../../api/operations/field.md#field-rescan). Five consecutive failures disable the plugin; see [Loading and failures](../start/loading.md#failures).
