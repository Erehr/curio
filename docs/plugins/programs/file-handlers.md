# File handlers

Use CLI extractors (`extractors[].command`) and CLI thumbnailers (`formats[].thumbnail.command`) when declarative rules (`png_text`, `regex`, `json`) or browser worker pages (`page`, `thumbnail_page`) are not enough, such as when decoding binary formats with Python or native tools. Both require `files.read` and `exec`.

## Extractor input

Declare a command extractor in `plugin.json`:

```json
{
  "extractors": [
    {
      "id": "cad-meta",
      "version": 1,
      "exts": ["step", "iges"],
      "command": ["python", "extract.py", "{input}", "{parts}"],
      "fields": ["cad_parts", "cad_units"]
    }
  ]
}
```

Curio invokes the command once per file during indexing (no `stdin`, **30-second timeout**) and substitutes two placeholders anywhere in the argument vector:

- **`{input}`**: Absolute disk path to the primary file.
- **`{parts}`**: JSON array of grouped companion files: `[{"role":"metadata","path":"..."}]`.

## Extractor output

Print a single flat JSON object to `stdout` mapping field keys to values (strings, numbers, booleans or arrays for `multi_select` and paired fields):

```python
import json, sys

path = sys.argv[1]
parts = json.loads(sys.argv[2]) if len(sys.argv) > 2 else []
# ... parse file ...
json.dump({
    "cad_parts": 14,
    "cad_units": "mm",
    "__title": "Gearbox Assembly"
}, sys.stdout)
```

- Curio applies only keys listed in `extractors[].fields`, plus `__title`, `__annotation` and `__source_url` for empty item metadata.
- Curio clears listed fields omitted from the JSON object on rescan while preserving hand-edited user overrides.
- Curio records output exceeding 4 MB, invalid JSON, non-zero exit codes or timeouts in **Settings › Fields** (`field.failures`).

## Thumbnail input

Declare a command thumbnailer on a format in `plugin.json`:

```json
{
  "formats": [
    {
      "exts": ["step", "iges"],
      "kind": "model",
      "thumbnail": {
        "command": ["python", "render_thumb.py", "{input}", "{output}", "{size}"]
      }
    }
  ]
}
```

Curio runs the command once per item needing a thumbnail (**90-second timeout**), substituting:

- **`{input}`**: Absolute path to the item file on disk.
- **`{output}`**: Destination file path where your program writes a PNG, JPEG or WebP thumbnail image.
- **`{size}`**: Target maximum edge dimension in pixels as a decimal integer string (for example `"512"`).

## Thumbnail output

- **Success**: Write a valid PNG, JPEG or WebP file to `{output}` and exit with code `0`. Curio reads `{output}`, stores it in the library thumbnail cache, extracts its color palette and removes the temporary output file.
- **Failure**: Print diagnostics to `stderr` (Curio writes `stderr` to `curio.log` under your plugin ID) and exit non-zero or leave `{output}` unwritten. Consecutive failures trigger the failure circuit breaker; see [Loading and failures](../start/loading.md#failures).
