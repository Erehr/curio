# Search

The toolbar search box (++ctrl+f++) combines prefix text search with a live editor for Curio's [Query language](../../api/query/syntax.md).

## Text search

Type plain words into the search box to narrow results as you type:

- **Prefix matching**: Typing `land` matches `landscape`, `landing` and `land_01.png`.
- **Multiple words**: Space-separated words require all terms (implicit `AND`). Wrap a phrase in quotes (`"blue sky"`) to match it as a unit.
- **Exclusions**: Prefix a word or quoted phrase with `-` (for example `portrait -draft`) to exclude matching assets.

## Search fields

Click the search options button on the left side of the search box to choose which fields plain words match:

- **Name**
- **Folder Name**
- **Extension**
- **Tags**
- **URL**
- **Note**
- **Indexed custom fields** (text, choice and URL fields such as Prompt, Author, Title or Camera)

Turn off noisy fields like **URL** or **Note** to keep plain-word searches focused on names and tags. Explicit field queries still search every field.

## Scope

By default, searching narrows the active scope in the **Library** sidebar: **All**, **Favorites**, a **Folder** (respecting **Show subfolder contents**), a **Smart folder**, a **Location** or subdirectory, a **Tag** or **Trash**. Switch to **All** to search the entire library.

## Expressions

When your search includes a field name and operator, Curio evaluates it with the [Query language](../../api/query/syntax.md):

```text
tag = concept AND rating >= 4
ext IN (png, webp) AND width >= 2560
favorite = true AND explicit = false
camera CONTAINS "Sony" AND added WITHIN 30d
lora = "detail_tweaker" WITH lora_strength >= 0.7
color = #3b9160
```

You can combine plain words and structured clauses in one query (`forest tag = reference rating >= 3`). See every built-in field and alias in the [Searchable field dictionary](../../api/query/fields.md).

## With filters

Active chips in the **Filter bar** combine with the search box using `AND`. For example, you can pick **Types: Image** and a color swatch in the **Filter bar** while typing `character -sketch` in the search box. Click **Lock filter** to keep active filter chips applied when you switch folders.

## Errors

- **Syntax errors**: Unmatched quotes, unknown operators or invalid dates and colors trigger an inline error message instead of empty results.
- **Disabled fields**: If a query references an unindexed custom field, Curio flags it so you can re-enable indexing in **Settings › Fields**.
