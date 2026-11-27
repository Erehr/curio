# Extractors

Keys inside `plugin.json`'s `extractors[]` array. See [Register a metadata reader](../contributions/extractors.md).

## extractors[].exts {#extractors-exts}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Lowercase file extensions without leading dots (for example `["png", "webp"]`) that this extractor processes. When omitted, extractors with only `png_text` or `regex` default to `["png"]`; `command`, `json` and `page` extractors match all extensions.

## extractors[].png_text {#extractors-png_text}
<div class="curio-meta"><span class="curio-badge">Record&lt;string, string&gt;</span><span class="curio-badge curio-badge--muted">Default: {} · Zero permissions</span></div>
Map of PNG text chunk keyword (`tEXt`, `zTXt` or `iTXt`) to target field key.

## extractors[].regex {#extractors-regex}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: [] · Zero permissions</span></div>
Regular expression rules extracting the first capture group from named PNG text chunks.

## extractors[].command {#extractors-command}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: [] · Requires files.read + exec</span></div>
Executable and arguments (for example `["python", "read.py", "{input}", "{parts}"]`) printing a flat JSON object of field values to `stdout` (30 s timeout). Relative executable paths resolve against the plugin folder.

## extractors[].json {#extractors-json}
<div class="curio-meta"><span class="curio-badge">object | null</span><span class="curio-badge curio-badge--muted">Default: null · Zero permissions</span></div>
Declarative JSON Pointer extraction over a grouped companion file (up to 8 MB).

## extractors[].page {#extractors-page}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null · Requires files.read</span></div>
Relative path to a hidden HTML extractor page served from the plugin's origin (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows) that handles [`plugin.extract`](../hooks/work.md#plugin-extract) via `curio.extractDone` (60 s timeout).

## extractors[].id {#extractors-id}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: "x&lt;index&gt;"</span></div>
Short reader identifier forming `<plugin_id>/<id>` in `Field.readers`, `field.failures` and `plugin.extract`. When omitted, Curio assigns `x1`, `x2` and so on by position.

## extractors[].version {#extractors-version}
<div class="curio-meta"><span class="curio-badge">integer</span><span class="curio-badge curio-badge--muted">Default: 0</span></div>
Reader version number. Incrementing `version` triggers a one-time automatic rescan of matching files per library.

## extractors[].fields {#extractors-fields}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Declared output field keys populated by this extractor. Required for `page` extractors and recommended for `command` extractors so fields that are no longer returned are cleared on rescan.

## extractors[].json.role {#extractors-json-role}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required inside json</span></div>
Companion role (declared in `formats[].companions`) to search for the JSON file (for example `"metadata"`).

## extractors[].json.file {#extractors-json-file}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Case-insensitive filename suffix filter matching the first companion of `role` whose name ends with `file` (for example `".civitai.info"`).

## extractors[].json.fields {#extractors-json-fields}
<div class="curio-meta"><span class="curio-badge">Record&lt;string, string&gt;</span><span class="curio-badge">Required inside json</span></div>
Map of output field key (or `__title`, `__annotation` or `__source_url` to fill empty item metadata) to RFC 6901 JSON Pointer (for example `"/civitai/trainedWords"`).

## extractors[].regex[].chunk {#extractors-regex-chunk}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required inside regex[]</span></div>
PNG text chunk keyword to search (for example `"parameters"`).

## extractors[].regex[].pattern {#extractors-regex-pattern}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required inside regex[]</span></div>
Regular expression whose first capture group becomes the extracted field value.

## extractors[].regex[].field {#extractors-regex-field}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required inside regex[]</span></div>
Target field key receiving the captured string.
