# Fields

Curio uses one unified typed field system for both custom metadata and properties read automatically from files by built-in and plugin readers. Every field has a stable key, a data type, an owner, visibility controls and an indexing switch under **Settings › Fields**.

## Custom fields

Open **Settings › Fields** (from the main menu › **Fields** or **Settings › Fields**). The top **Custom** card manages fields you create yourself:

1. Enter a **Label** (required).
2. Optionally enter a **key**. If left blank, Curio derives one from the label using lowercase letters, digits, `_` and `.`.
3. Choose one of the 11 supported types from **Type**.
4. For **Single choice** or **Multiple choice**, optionally enter comma-separated allowed options (`a, b, c`). Leave options empty to allow free-form values.
5. Click **Create** (or press ++enter++).

Field keys cannot collide with built-in query properties (`name`, `tag`, `rating`, `size` and similar). Click any custom field row to expand its settings and inspect its key, type and options. From there you can toggle **Inspector and columns**, choose its **Filter** control or promote it to **With the main filters**. Use the row's switch to turn a field off without losing its stored values.

!!! warning "Deleting a custom field removes all its values"
    Clicking the trash button on a custom field deletes the field definition and permanently removes all of its stored values from every asset. Switch the field off instead if you might need that data later.

## Field types

Every field stores validated, typed values in SQLite so sorting, range queries and filters behave consistently:

| UI type label | API type key | Stored values and editor behavior |
|---|---|---|
| **Text** | `text` | Free text. Also sorts and filters numerically when a string parses as a number. Switches to a multi-line box above 48 characters or in block/code views. |
| **Whole number** | `integer` | Rounded integer stored numerically. Accepts comma-separated numbers when editing multi-value lists. |
| **Decimal number** | `float` | Floating-point number displayed up to 6 decimal places. |
| **Yes / No** | `boolean` | Stored as `1` or `0` and edited with an on/off switch in the Inspector. |
| **Date** | `date` | Calendar date (`YYYY-MM-DD`), stored with both display text and millisecond timestamp. |
| **Date and time** | `datetime` | Timestamp (`YYYY-MM-DD HH:MM`), stored with both display text and millisecond timestamp. |
| **Single choice** | `single_select` | Single value validated case-insensitively against defined options. Shows a clearable dropdown when options are defined, or a text box otherwise. |
| **Multiple choice** | `multi_select` | List of values shown as removable chips. Type comma-separated entries to add several at once. |
| **JSON** | `json` | Validated JSON text in a monospace multi-line editor. |
| **URL** | `url` | Web address (`https://example.com`). |
| **File reference** | `file_ref` | File path string. |

!!! note "Type stability"
    A field's type cannot be changed once assets store values for it. Curio keeps the original type so existing data is never reinterpreted or lost.

## Ownership

Every field belongs to the source that defined it:

- **Custom**: Fields you create in **Settings › Fields** or through the API. You can edit, switch off or delete them.
- **Built-in format readers**: Shipped with Curio for **Images**, **Audio**, **Video** and **Fonts**, grouped into one card per format family in **Settings › Fields**. You can switch them on or off, but not delete them.
- **Plugin fields**: Declared by enabled plugins (such as AI generation, Reader or 3D models) and grouped under each plugin's card and icon.
- **Shared fields**: When two plugins or built-in format families declare the same key with the same type, both readers populate the same searchable field. For example, **Video** and **Audio** share `audio_codec`, `bit_rate`, `channels` and `sample_rate`, while **Images** and **Audio** share `artist`. Declaring an existing key with a different type is refused.

## Overrides

Custom fields and editable plugin fields can be edited in the Inspector, in **Batch edit**, in **Actions** (**Set field**) or through the API (`item.update`).

When you edit a field that is also populated by a file reader, Curio records a **manual override** for that asset and field:

- Future background metadata rescans and file re-reads skip overridden fields so your manual edits stay intact.
- Overridden fields show a pencil-check button (**Edited · Restore file value**) in the Inspector. Click it to clear the override so the value read from the file returns on rescan.
- Running **Clean up** on switched-off fields removes only values read from files and keeps manual edits.

## Visibility

Expand any active field row in **Settings › Fields** to control where and how it appears:

| Setting | What it controls |
|---|---|
| **Shown › Inspector and columns** | Controls whether the field appears in the Inspector (for custom fields inside **Properties**) and as a List view column. You can also place any field as its own Inspector widget in **Edit layout**. |
| **Filter** | Sets the filter popover style: **Auto** (chosen from the field type), **List** (checklist where each value can be included, excluded or ignored), **Range** (min/max numeric or date bounds), **Keywords** (comma-separated word matching) or **Yes / No** (has a value or empty). |
| **Listed › With the main filters** | Moves the field out of **Custom fields** into the main filter bar and smart folder rule list, and pins or unpins its chip on the filter bar. |

## Pairs & groups

Plugins can declare two relationships between related fields (see [Field declarations](../../plugins/manifest/fields.md)):

- **Paired fields**: Links a secondary field to a primary field, such as `LoRA strength` paired with `LoRA` (multi-value parent) or `Number in series` paired with `Series` (single-value parent):
    - In the Inspector, paired values appear beside each primary value in parentheses (`my_lora (0.8)`).
    - In **Settings › Fields**, a paired secondary field displays *"In the &lt;Parent&gt; filter, beside each value"* instead of its own filter style control.
    - Multi-value paired fields are filtered inside their parent's list popover (with Min/Max inputs beside each value). Single-value paired fields work both inside their parent filter and standalone.
    - Paired secondary fields are hidden from **Sort by** and **Group by** menus. Sorting by the primary field sorts case-insensitively on the primary value and tie-breaks ascending on the paired field (so sorting by `Series` orders books by `Number in series`).
    - Switching off either side of a pair switches off its partner too.
- **Grouped numeric filters**: Combines several numeric fields into one shared filter popover (such as camera exposure settings or generation parameters). Enter min and max bounds across related fields in one place.

## Indexing

Every field card in **Settings › Fields** displays an on/off switch on every field row, including custom fields, manual plugin fields and file-reading fields. A card's heading shows how many fields are on (for example `9/14` when 5 are switched off).

- **Switching a field on**: Custom fields and manual plugin fields turn on immediately. For file-reading fields, Curio asks whether to **Scan now** or **Later**. **Scan now** queues a background rescan for files missing that field and its pair. **Later** marks the field `Not scanned · Scan now` and indexes new and changed files automatically until you backfill existing files.
- **Switching a field off**: Curio lists everything referencing that field and its pair in a confirmation dialog: smart folders, plugin filters, plugin sorts, pinned filter chips, List view columns and the current sort order. Those references stay intact and **stored values stay in the database** so turning the field back on restores them immediately. Queries, filters, sorts and grouping cannot use a field while it is off.
- **On-demand inspection while off**: Selecting a single asset still reads switched-off file-backed fields directly from disk on demand and shows them in the Inspector as read-only, marked **Not indexed**.

## Diagnostics

Each source card in **Settings › Fields** tracks indexing health live:

- **Waiting count**: Shows how many eligible files are queued for extraction, along with a progress bar and **Stop** button while a rescan runs.
- **Rescan**: Click the refresh icon on a card header (to rescan all active reader fields in that group) or **Rescan** inside an expanded field row, then choose:
    - **Missing only**: Scans only files without a recorded read for that reader.
    - **Everything**: Re-reads all matching files, replaces extracted values and clears values the file no longer holds. Manual overrides stay intact.
- **Failed files and retry**: When a reader fails on one or more files, a red **N failed** link appears on the group header and field row. Click it to open **Failed files** (listing file name, reader and error message) and click **Try again** to re-queue only those failed assets.
- **Clean up**: When switched-off built-in or plugin fields still hold extracted values, a footer bar shows *"N values kept for fields switched off"* with a **Clean up** button. Clicking it previews the count per field and removes only file-extracted values while keeping manual edits. Fields from uninstalled plugins (marked **plugin gone**) start unchecked by default so you do not discard their data accidentally.

For automation and scripting, see the [Fields and indexing API](../../api/operations/field.md) and [Register indexable fields](../../plugins/contributions/fields.md).
