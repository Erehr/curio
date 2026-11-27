# Batch rename

**Batch rename** updates names for multiple assets in one pass using a sequence of operations, metadata tokens, live preview, saved presets and undo.

## Selection

Select two or more assets in the grid and press ++f2++ to open **Batch rename**. Pressing ++f2++ on one asset focuses **Name** in the Inspector instead. You can also choose **Rename all** from the **Item menu** or add **Batch rename** to your toolbar in **Edit layout**.

The Batch rename window lists seven rename operations on the left and a live before-and-after preview table on the right.

## Text rules

Operations run top to bottom on each asset's name without its extension. Each active operation runs on the output of the one before it. Click a row to configure an operation and toggle its switch to turn it on or off:

| Operation | What it does |
|---|---|
| **New name** | Replaces the name using literal text and metadata tokens, such as `<FolderName>_<Inc Nr:001>`. |
| **Add** | Inserts text and tokens at the start, at the end, at a character position or before or after the first match of text. |
| **Replace** | Replaces literal text or a regular expression (with capture groups such as `$1`) across all matches, the first match or the last match. Replacement text accepts tokens. |
| **List replace** | Runs an ordered table of find-and-replace pairs in sequence. |

## Clean & case

| Operation | What it does |
|---|---|
| **Remove** | Deletes matched text (`*` wildcard supported), characters by position, a character set, digits, symbols, accents or text inside brackets. |
| **Case** | Converts names to `lower case`, `UPPER CASE`, `Title Case`, `Sentence case` or `iNVERT cASE`. You can list words to keep as written (such as `DVD`, `of` or `UI`) and set a separate case rule for the extension. |
| **Trim** | Strips specified characters (such as spaces, underscores or hyphens) from the start, the end or both ends. Trim runs last. |

## Tokens

Tokens are angle-bracket placeholders (`<Token>` or `<Token:option>`) that insert each asset's metadata, filename parts or counter values. Click the `</>` (**Insert tag**) button in a text field to search and insert tokens:

| Token | Output |
|---|---|
| `<Name>` | Current base name |
| `<Ext>` | File extension |
| `<Word:2>`, `<Word:-1>` | Nth word (negative indexes count from the end) |
| `<Substr:1:5>` | Character slice starting at position 1 with length 5 |

## Counters & fields

| Token | Output |
|---|---|
| `<Inc Nr>`, `<Inc Nr:001:2>` | Sequential counter across selected assets with optional zero-padding and step (`001`, `003`, `005`) |
| `<Inc NrDir>` | Counter that resets inside each asset's first folder (unfiled assets share one group) |
| `<Inc Alpha>`, `<Inc Alpha:A>` | Alphabetic sequence (`a`, `b`, `c` or uppercase `A`, `B`, `C`) |
| `<Num Items>`, `<Num Files>` | Total selected asset count or count within the asset's folder |
| `<FolderName>`, `<Folders>` | First folder name or all folder names |
| `<Tags>` | Assigned tags |
| `<Kind>`, `<Rating>` | Asset type or star rating |
| `<Width>`, `<Height>`, `<Width:4>` | Pixel dimensions with optional zero-padding width |
| `<Field:key>` | Value of any custom or plugin field by key (such as `<Field:camera>` or `<Field:series>`) |
| `<Date>`, `<Time>`, `<Added>`, `<Modified>` | Timestamps with optional format strings (such as `<Date:yyyymmdd>`) |

## Preview

The preview table on the right updates live as you edit:

- **Unchanged or empty names**: Shown in muted text and skipped. Curio never renames an asset to an empty string.
- **Skip individual assets**: Click the `×` button on any row to leave that asset out of the batch. Click **Include all** in the footer to restore skipped rows.
- **Warnings**: Flags duplicate resulting names within the batch or names longer than 255 characters before you apply.
- **Storage behavior**: Batch rename updates names in the library index and renames files on disk according to your storage access rules. See [File operations and trash](../organize/file-actions.md).

## Presets & undo

- **Presets**: Curio remembers your last-used configuration. Use the footer controls to save named presets, load saved presets or delete ones you no longer need.
- **Apply & undo**: Click **Rename** (or press ++ctrl+enter++) to apply all valid changes in one batch. An **Undo** button appears in the footer for 30 seconds so you can revert the entire batch.
