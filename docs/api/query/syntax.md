# Query syntax

Curio uses one query language across the desktop search box, filter bar chips, smart folders, plugin filters, [`item.query`](../operations/item-reads.md#item-query), [`item.count`](../operations/item-reads.md#item-count), [`query.validate`](../operations/query.md#query-validate), [`query.facets`](../operations/query.md#query-facets) and MCP `item_query`. Every query compiles directly to parameterized SQL, so nothing is filtered in memory.

```text
model = "Anima" AND steps <= 12 AND cfg = 1
prompt CONTAINS "blue hair" AND lora = detail WITH lora_strength >= 0.7
(tag = cat OR tag = dog) AND rating >= 4 AND NOT ext = gif
shape = portrait AND width >= 2000 AND added WITHIN 30d
favorite = true AND explicit = false
tags HAS ALL (sky, sea) AND folder IN ("Concept art", References)
color ~ #3366ff/75 AND NEAR(52.23, 21.01, 10)
```

## Grammar

```ebnf
expr       := and (OR and)*
and        := unary ([AND] unary)*                 (* juxtaposition implies AND *)
unary      := NOT unary | '-' unary | primary
primary    := '(' expr ')' | comparison (WITH comparison)* | place | word | "quoted text"
place      := NEAR(lat, lon, km) | INSIDE(south, west, north, east)
comparison := field op value
            | field ':' [op] value                 (* shorthand *)
            | field IS [NOT] EMPTY
            | field [NOT] IN (v, v, ...)
            | field BETWEEN v AND v
            | field HAS [ALL|NONE|ANY] (v, ...)
            | field WITHIN duration
```

## Text search

Bare words (`sunset`) and double-quoted phrases (`"blue sky"`) query Curio's SQLite FTS5 full-text index using **prefix matching**.

- **Indexed columns**: Matches item name, notes (`annotation`), source URL, tag names and indexed custom fields of type `text`, `single_select`, `multi_select` and `url`.
- **Desktop vs API scope**: In the desktop search box, the field-scope menu limits which columns plain text searches. In API calls such as `item.query`, plain text always searches every indexed text column.

## Fields

- **Built-in fields**: Refer to any built-in field by its canonical name or alias (case-insensitive). See [Query fields](fields.md).
- **Custom and plugin fields**: Refer to any custom field by its `key` or by its label with spaces and hyphens replaced by underscores (`LoRA strength` -> `lora_strength`).
- **Name collisions**: Built-in names take precedence over custom keys. Prefix colliding custom fields with `field.<key>` (for example `field.size`).
- **Colon shorthand**: `tag:sky`, `ext:png`, `rating:>=4`, `steps:<=12` and `width:>2000` work as shorthand for `=`, `>=`, `<=` and `>`.

## Boolean logic

- **Precedence** (highest to lowest): Parentheses `( ... )`, then `NOT` or unary `-`, then `AND` (or whitespace juxtaposition), then `OR`.
- **Implicit AND**: `tag = sky rating >= 4` means `tag = sky AND rating >= 4`.
- **Unary minus**: `-draft`, `-"work in progress"` and `-tag:wip` negate the following term or comparison.

## Comparisons

| Operator | Applies to | Semantics |
|---|---|---|
| `=`, `!=` | All types | Case-insensitive match for text; numeric equality with float tolerance on custom floats. On custom fields, `!=` also matches items with no value. |
| `<`, `<=`, `>`, `>=` | Numbers, sizes, durations, dates, ratings | Numeric or chronological comparison. |
| `BETWEEN a AND b` | Numbers, sizes, durations, dates | Inclusive range (`>= a AND <= b`). |
| `IS EMPTY`, `IS NOT EMPTY` | Text, tags, folders, `folder_color`, `part`, custom fields | Matches missing or empty values (`model IS EMPTY`, `annotation IS NOT EMPTY`). |

## String ops

- **`CONTAINS`, `NOT CONTAINS`, `STARTS WITH`, `ENDS WITH`**: Case-insensitive substring checks on text fields (`prompt CONTAINS "watercolor"`, `url STARTS WITH "https://github.com"`).
- **`MATCHES "regex"`**: Case-insensitive regular expression match (`name MATCHES "^IMG_\\d{4}"`). Invalid regexes fail validation.

## List ops

- **`IN (v1, v2, ...)` / `NOT IN (v1, v2, ...)`**: Matches when the field equals (or does not equal) any value in the non-empty list (`ext IN (png, webp, avif)`). An empty `IN ()` is a syntax error.
- **`HAS ANY` / `HAS ALL` / `HAS NONE`**: Multi-value set operators for `tags`, `folders` and `multi_select` custom fields. `tags HAS ALL (sky, sea)` requires both tags; `tags HAS NONE (draft, wip)` excludes items carrying either tag.

## Paired WITH

`WITH` joins two or more comparisons on paired multi-value custom fields so every clause matches the same list entry:

```text
lora = "detail_tweaker" WITH lora_strength BETWEEN 0.5 AND 1.0
```

Without `WITH`, `lora = "detail_tweaker" AND lora_strength >= 0.5` would match an item using `detail_tweaker` at `0.2` alongside another LoRA at `0.8`.

Each clause after `WITH` must be a single-value predicate (`=`, `!=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `CONTAINS`, `IN` and similar), never `IS EMPTY` or `HAS`. When the word `with` is not followed by a field condition, Curio treats it as a plain search word.

## Dates

Date fields (`added`, `modified`, `created`, `edited` and custom `date` or `datetime` fields) support:

- **Aliases**: `BEFORE` (`<`), `AFTER` (`>`) and `ON` (`=`).
- **Literals**: `YYYY-MM-DD` (`2026-11-24`), `YYYY-MM-DD HH:MM`, `YYYY-MM` (whole month), `YYYY` (whole year), `today` and `yesterday`.
- **Whole-period semantics**: `added = 2026` spans all of 2026; `added <= 2026-05` includes all of May 2026; `added = 2026-11-24` covers the entire calendar day.
- **Relative durations**: Units `s`, `m`, `h`, `d` (days), `w` (weeks) and `y` (years) resolve to a cutoff timestamp in the past (`now - duration`):
  - `added WITHIN 30d` and `added > 7d` (or `added AFTER 7d`) match **newer** items from within the last 30 or 7 days.
  - `added < 30d` (or `added BEFORE 30d`) matches **older** items from more than 30 days ago.

## Units

- **File sizes**: `kb`, `mb`, `gb`, `tb` (case-insensitive) or raw bytes (`size >= 5mb`, `size BETWEEN 100kb AND 2mb`).
- **Durations**: `ms`, `s`, `m`, `h` (`duration <= 2m`, `duration >= 30s`).
- **Aspect ratios (`aspect`)**: Decimals or fractions (`aspect = 16/9`, `aspect >= 1.5`). Equality uses a **±0.015 tolerance** matching the Filter bar's 4:3 and 16:9 chips, and excludes items with unknown dimensions.
- **Shapes (`shape`)**: `landscape`, `portrait`, `square`, `wide` (panorama >= 2.35:1) and `tall` (vertical panorama <= 1:2.35).

## Colors

| Expression | Meaning |
|---|---|
| `color ~ #3366ff` | **Similar**: At least one palette swatch covers >= 4% of the image within **ΔE2000 <= 16** (or exact hex match). |
| `color = #3366ff` | **Accurate**: Within **ΔE2000 <= 8** (>= 4% coverage). |
| `color ~ #3366ff/75` | **Custom accuracy (`0` to `100`)**: Interpolates the ΔE2000 threshold from `28` (accuracy `0`) down to `4` (accuracy `100`); `50` matches default `~`. |
| `color = red` | Named presets (`red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `pink`, `black`, `white`, `brown`). |
| `color = grayscale` | Every palette swatch covering >= 0.5% has CIELAB chroma below `6`. |

## Geography

Two spatial functions query `gps_lat` and `gps_lon` populated by the built-in Camera EXIF reader:

- **`NEAR(lat, lon, km)`**: Matches items photographed within `km` kilometers of `(lat, lon)`. Uses a local planar approximation accurate within a fraction of a percent up to several hundred kilometers. Example: `NEAR(52.23, 21.01, 10)`.
- **`INSIDE(south, west, north, east)`**: Matches items inside a latitude/longitude bounding box. When `west > east`, the longitude span wraps across the 180° antimeridian. Example: `INSIDE(49, 14, 55, 24)`.
- **Items without GPS**: Items lacking GPS coordinates never match `NEAR(...)` or `INSIDE(...)`, so they match `NOT NEAR(...)`.

## Nulls & errors

- **Disabled file-reading fields**: Querying a disabled file-reading field (such as `vae = x` in **Settings › Fields**) fails with `Field '<key>' isn't indexed (Settings › Fields)`. Curio never silently returns zero items.
- **Validation**: Call [`query.validate`](../operations/query.md#query-validate) before saving or running untrusted queries to check syntax errors and match counts.
