# Query fields

Every built-in field and alias recognized by Curio's query language, plus how custom and plugin fields work.

## Identifiers

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `name` | `display`, `display_name` | Text (display title) | `name CONTAINS "poster"` |
| `filename` | `file`, `file_name` | Text (actual filename on disk) | `filename MATCHES "^IMG_\\d+"` |
| `ext` | `extension`, `type`, `format` | Text (lowercase, without dot) | `ext IN (png, webp, avif)` |
| `kind` | `media` | Enum: `image`, `video`, `audio`, `document`, `code`, `link`, `archive`, `font`, `model`, `other` | `kind = code OR kind = link` |
| `mime` | - | Text (MIME type) | `mime STARTS WITH "video/"` |
| `id` | `uid` | Text (UUIDv7) or integer row ID | `id = "0194..."` |

## Media

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `size` | `filesize`, `file_size` | Bytes or `kb`, `mb`, `gb`, `tb` | `size >= 10mb` |
| `width` | - | Integer (pixels; images and video) | `width >= 3840` |
| `height` | - | Integer (pixels; images and video) | `height BETWEEN 1080 AND 2160` |
| `aspect` | `ratio`, `aspect_ratio` | Float or `w/h` fraction (±0.015 on `=`) | `aspect = 16/9` |
| `duration` | `length` | Seconds or `ms`, `s`, `m`, `h` | `duration <= 30s` |
| `animated` | `animation` | Boolean (`true`, `false`, `1`, `0`) for animated GIF, WebP and APNG | `ext = webp AND animated = true` |

## Times

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `created` | `btime`, `ctime`, `date_created` | Date, DateTime or relative | `created = 2025` |
| `modified` | `mtime`, `date_modified` | Date, DateTime or relative | `modified AFTER 2026-01-01` |
| `added` | `imported`, `indexed`, `date_added`, `date_imported` | Date, DateTime or relative | `added WITHIN 14d` |
| `edited` | `updated` | Date or DateTime (last metadata edit in Curio) | `edited ON today` |

## Metadata

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `rating` | `stars`, `star` | Integer `0` to `5` | `rating >= 4` |
| `favorite` | `favourite`, `fav` | Boolean (`true`, `false`, `yes`, `no`, `1`, `0`) | `favorite = true` |
| `explicit` | `nsfw` | Boolean (`true`, `false`, `yes`, `no`, `1`, `0`) | `explicit = false` |
| `annotation` | `note`, `notes`, `description`, `comment` | Text | `annotation IS NOT EMPTY` |
| `url` | `source`, `source_url`, `link` | Text | `url CONTAINS "pixiv.net"` |
| `tag` | `tags` | Multi-value text (tag names) | `tags HAS ALL (sky, cloud)` |
| `tag_count` | `tagcount` | Integer (number of attached tags) | `tag_count = 0` |
| `folder` | `folders`, `collection`, `collections` | Multi-value text (folder names) or folder ID | `folder IN ("Concept art", Finals)` |
| `folder_color` | `folder_colour` | Hex `#rrggbb` or `"var(--accent)"` (direct folders only) | `folder_color = "#e5686b"` |

## Storage

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `path` | `folder_path` | Text (full file path on disk) | `path CONTAINS "RAW"` |
| `hash` | `content_hash` | Text (BLAKE3 content hash) | `hash IS NOT EMPTY` |
| `status` | `availability` | Enum: `ok`, `offline`, `missing` | `status = missing` |
| `root` | `location` | Location name or numeric root ID | `root = "NAS Photos"` |
| `deleted` | `trash`, `trashed` | Boolean (whether the item is in Trash) | `deleted = true` |

## Colors

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `color` | `colour` | `#rrggbb`, `#rrggbb/<0-100>`, color name or `grayscale` | `color ~ #3b9160/70` |
| `shape` | `orientation` | `landscape`, `portrait`, `square`, `wide`, `tall` | `shape = portrait` |

## Parts

| Canonical field | Aliases | Value type | Example |
|---|---|---|---|
| `parts` | `part_count` | Integer (count of files grouped into the item) | `parts > 0` |
| `part` | - | Multi-value text (companion roles: `motion`, `edits`, `metadata`, `raw`, `preview`, `materials`, `buffers`, `user`) | `part = motion` (Live Photos) |

## Custom fields

Every user-defined, built-in reader and plugin custom field is queryable automatically:

- **By key**: `camera = "Sony A7 IV"`, `steps <= 12`, `font_weight >= 700`, `author CONTAINS "Le Guin"`.
- **By normalized label**: Spaces and hyphens in the field label map to `_` (case-insensitive), so `Model type` matches `model_type`.
- **Shadowing & `field.<key>`**: Built-in names take precedence over custom keys. When a custom key matches a built-in name or alias (`type`, `size`, `link`, `comment`), prefix it with `field.<key>` (for example `field.comment CONTAINS "approved"`).

## Operators

| Custom field type | Supported operators | FTS bare-word search |
|---|---|---|
| `text`, `url`, `file_ref` | `=`, `!=`, `CONTAINS`, `NOT CONTAINS`, `STARTS WITH`, `ENDS WITH`, `MATCHES`, `IN`, `NOT IN`, `IS [NOT] EMPTY` | `text` and `url` included |
| `integer`, `float` | `=`, `!=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `IN`, `NOT IN`, `IS [NOT] EMPTY`, `WITH` (when paired) | No |
| `boolean` | `=`, `!=` (`true`, `false`, `yes`, `no`, `1`, `0`), `IS [NOT] EMPTY` | No |
| `date`, `datetime` | `=`, `!=`, `<`, `<=`, `>`, `>=`, `BEFORE`, `AFTER`, `ON`, `BETWEEN`, `WITHIN`, `IS [NOT] EMPTY` | No |
| `single_select` | `=`, `!=`, `IN`, `NOT IN`, `CONTAINS`, `IS [NOT] EMPTY` | Yes |
| `multi_select` | `=`, `!=`, `IN`, `NOT IN`, `HAS ANY`, `HAS ALL`, `HAS NONE`, `CONTAINS`, `IS [NOT] EMPTY`, `WITH` (when paired) | Yes |
| `json` | `IS [NOT] EMPTY`, `CONTAINS` | No |

File-reading custom fields must have **`indexed: true`** (enabled in **Settings › Fields**) before you can query them. Naming an unindexed file-reading field returns a validation error directing you to **Settings › Fields**.
