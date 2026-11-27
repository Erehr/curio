# Organizing

Types for organizing items and inspecting custom field schemas, coverage and facets.

## Folder {#folder}

A library folder returned by [`folder.list`](../operations/folder.md#folder-list), [`folder.get`](../operations/folder.md#folder-get) (which adds `children: number[]`) and [`folder.recent`](../operations/folder.md#folder-recent):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric folder ID. |
| `parentId` | `number \| null` | Parent folder ID, or `null` for top-level folders. |
| `name` | `string` | Folder name. |
| `color` | `string \| null` | CSS color (`#rrggbb` or `var(--accent)`), or `null`. |
| `sortOrder` | `number` | Display order among siblings. |
| `count` | `number` | Number of direct items in this folder. |
| `icon` | `string \| null` | Tabler icon name, or `null`. |
| `autoTags` | `string[]` | Tags applied to new items added to this folder. |

## SmartFolder {#smartfolder}

A saved dynamic query returned by [`smartFolder.list`](../operations/smart-folder.md#smartfolder-list) and [`smartFolder.get`](../operations/smart-folder.md#smartfolder-get):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric smart folder ID. |
| `name` | `string` | Display name. |
| `query` | `string` | [Query language](../query/syntax.md) expression (empty string for ruleless container or pure-grouping smart folders). |
| `sort` | `string \| null` | Optional saved sort specification. |
| `icon` | `string \| null` | Tabler icon name, or `null`. |
| `parentId` | `number \| null` | Parent smart folder ID (`null` at top level). |
| `sortOrder` | `number` | Order among siblings. |
| `groupBy` | `string \| null` | Virtual subfolder grouping (`year:added`, `month:created`, `rating`, `kind`, `ext`, `tag`, `field:<key>` or `null`). |
| `count` | `number \| null` | Matching item count (populated by `smartFolder.get`; may be `null` in `smartFolder.list`). |

## Tag {#tag}

A tag returned by [`tag.list`](../operations/tag.md#tag-list) and [`tag.get`](../operations/tag.md#tag-get):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric tag ID. |
| `name` | `string` | Tag name (unique case-insensitively). |
| `groupId` | `number \| null` | Primary tag group ID whose color this tag inherits when `color` is `null`. |
| `color` | `string \| null` | Explicit CSS color overriding group color, or `null`. |
| `count` | `number` | Number of non-trashed items carrying this tag. |
| `starred` | `boolean` | Whether the tag is starred in the Tag Manager and pickers. |
| `groupIds` | `number[]` | All tag group IDs this tag belongs to. |

## TagGroup {#taggroup}

A tag group returned by [`tagGroup.list`](../operations/tag-group.md#taggroup-list):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric group ID. |
| `name` | `string` | Group name. |
| `color` | `string \| null` | CSS color inherited by member tags. |
| `icon` | `string \| null` | Tabler icon name. |
| `description` | `string \| null` | Optional description text. |

## Field {#field}

A custom or file-reading metadata field definition returned by [`field.list`](../operations/field.md#field-list) and [`field.get`](../operations/field.md#field-get):

| Property | Type | Notes |
|---|---|---|
| `id` | `number` | Numeric field ID. |
| `key` | `string` | Unique machine key used in `item.update`, `ItemWithFields.fields` and queries. |
| `label` | `string` | Display label shown in the UI. |
| `type` | `'text' \| 'integer' \| 'float' \| 'boolean' \| 'date' \| 'datetime' \| 'single_select' \| 'multi_select' \| 'json' \| 'url' \| 'file_ref'` | Storage and validation type. |
| `description` | `string` | Help text shown in Settings and tooltips. |
| `defaultValue` | `string \| null` | Optional default value. |
| `options` | `string[]` | Allowed choices for `single_select` and `multi_select`. |
| `validation` | `{ min?: number; max?: number; maxLength?: number; pattern?: string; required: boolean }` | Validation constraints. |
| `format` | `string` | Optional display format hint. |
| `widget` | `string` | Inspector editor hint (`row`, `block` or `code`). |
| `owner` | `string` | `'user'`, `'builtin.format.<family>'` or the defining plugin ID. |
| `visible` | `boolean` | Whether shown in the Inspector and List column picker. |
| `sortOrder` | `number` | Display order. |
| `ownFilter` | `boolean` | Whether listed as a top-level filter chip and standalone Smart Folder rule. |
| `filterKind` | `string` | `'list'`, `'range'`, `'text'`, `'has'` or `''` (auto by type). |
| `indexed` | `boolean` | Whether Curio stores values and includes the field in search, filters, sorts and columns. |
| `pair` | `string \| null` | Primary field key this field pairs with (e.g. `lora_strength` pairs with `lora`). |
| `filterGroup` | `string \| null` | Shared numeric range group name (e.g. `"Model size (mm)"`). |
| `readers` | `string[]` | Reader IDs that populate this field from files (empty for manual user fields). |
| `needsScan` | `boolean` | `true` when switched on without having scanned existing files yet. |

## FieldCoverage {#fieldcoverage}

Per-field extraction progress returned by [`field.coverage`](../operations/field.md#field-coverage):

| Property | Type | Description |
|---|---|---|
| `key` | `string` | Field key. |
| `withValue` | `number` | Items currently holding a stored value. |
| `applies` | `number` | Total files matching the field's active readers. |
| `waiting` | `number` | Files queued or awaiting extraction. |
| `failed` | `number` | Files on which a reader reported an error. |

## FieldFacets {#fieldfacets}

Value distribution returned by [`field.facets`](../operations/field.md#field-facets):

```typescript
interface FieldFacets {
  values: [string | number, number][]; // [value, itemCount] pairs
  set: number;                         // items with any value for this field
  total: number;                       // total items matched by query
}
```

## Facets {#facets}

Aggregate counts returned by [`query.facets`](../operations/query.md#query-facets):

```typescript
interface Facets {
  rating: Record<string, number>;
  shape: { landscape: number; portrait: number; square: number; wide: number; tall: number; '4:3': number; '16:9': number };
  kinds: Record<string, number>;
  exts: Record<string, number>;
  animatedExts: Record<string, number>;
  animated: number;
  total: number;
}
```
