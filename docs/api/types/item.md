# Items

Core types returned when querying, reading or adding items.

## Kind {#kind}

Item family string returned on `Item.kind`:

```typescript
type Kind =
  | 'image'
  | 'video'
  | 'audio'
  | 'document'
  | 'code'
  | 'link'
  | 'archive'
  | 'font'
  | 'model'
  | 'other'
```

## Item {#item}

Compact item record returned by [`item.query`](../operations/item-reads.md#item-query), [`folder.items`](../operations/folder.md#folder-items), [`smartFolder.items`](../operations/smart-folder.md#smartfolder-items) and [`selection.get`](../operations/selection.md#selection-get).

| Property | Type | Notes |
|---|---|---|
| `id` | `string` | Stable UUIDv7 identifier across moves, renames and relinks. |
| `name` | `string` | Display title (falls back to `fileName` when no custom title is set). |
| `fileName` | `string` | Actual filename on disk including extension. |
| `ext` | `string` | Lowercase extension without leading dot (`"png"`, `"mp4"`, `"url"`). |
| `kind` | [`Kind`](#kind) | Item family. |
| `size` | `number` | File size in bytes. |
| `width` | `number \| null` | Pixel width for images and videos, or `null` for non-visual items. |
| `height` | `number \| null` | Pixel height for images and videos, or `null` for non-visual items. |
| `duration` | `number \| null` | Media length in seconds, or `null` for non-timed formats. |
| `rating` | `number` | Star rating from `0` to `5`. |
| `favorite` | `boolean` | Whether the item is marked as a favorite. |
| `explicit` | `boolean` | Whether the item is marked as explicit (blurred in grids). |
| `annotation` | `string` | Notes text (empty string when unset). |
| `url` | `string` | Source URL (for `.url` link items, the target web address). |
| `tags` | `string[]` | Assigned tag names. |
| `folders` | `{ id: number; name: string }[]` | Library folders containing the item. |
| `status` | `'Available' \| 'Storage offline' \| 'Missing'` | File availability on disk. |
| `imported` | `number` | Import timestamp (milliseconds since Unix epoch). |
| `modified` | `number \| null` | File modification timestamp on disk (milliseconds since Unix epoch). |
| `parts` | `number` | Count of grouped member files attached to this item (`0` when ungrouped). |
| `partOf` | `string \| undefined` | Present only when this record is a grouped member: UUID of the parent item. |
| `partRole` | `string \| null \| undefined` | Present only on grouped members: companion role (e.g. `"motion"`, `"raw"`, `"user"`), or `null` when unset. |
| `path` | `string \| undefined` | Full disk path. Included **only** when the caller holds `files.read`. |

```json
{
  "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
  "name": "Forest lighting study",
  "fileName": "forest_study.png",
  "ext": "png",
  "kind": "image",
  "size": 3145728,
  "width": 2560,
  "height": 1440,
  "duration": null,
  "rating": 5,
  "favorite": true,
  "explicit": false,
  "annotation": "Warm bounce light reference",
  "url": "https://example.com/art/99",
  "tags": ["environment", "lighting"],
  "folders": [{ "id": 4, "name": "References" }],
  "status": "Available",
  "imported": 1763900000000,
  "modified": 1763890000000,
  "parts": 0,
  "path": "C:\\Users\\You\\Libraries\\Design.curio\\files\\References\\forest_study.png"
}
```

## ItemWithFields {#itemwithfields}

Extended item record returned by [`item.get`](../operations/item-reads.md#item-get) and [`item.getMany`](../operations/item-reads.md#item-getmany). Includes every property of [`Item`](#item) plus:

| Property | Type | Notes |
|---|---|---|
| `fields` | `Record<string, unknown>` | Map of custom and plugin field keys to typed values (scalars, arrays, booleans, numbers or parsed JSON objects). |
| `parts` | `number \| ItemMember[]` | `0` when the item has no grouped members, or an array of member descriptors when `parts > 0`. |

Each grouped member entry follows `ItemMember`:

```typescript
interface ItemMember {
  id: string;
  role: string | null;
  name: string;
  ext: string;
  size: number;
  status: 'Available' | 'Storage offline' | 'Missing';
}
```

```json
{
  "id": "0194b7a2-8c1e-7a40-b2d1-3f9e81c4a012",
  "name": "Forest lighting study",
  "fileName": "forest_study.png",
  "ext": "png",
  "kind": "image",
  "size": 3145728,
  "width": 2560,
  "height": 1440,
  "duration": null,
  "rating": 5,
  "favorite": true,
  "explicit": false,
  "annotation": "Warm bounce light reference",
  "url": "https://example.com/art/99",
  "tags": ["environment", "lighting"],
  "folders": [{ "id": 4, "name": "References" }],
  "status": "Available",
  "imported": 1763900000000,
  "modified": 1763890000000,
  "fields": {
    "model": "AnimaXL",
    "steps": 12,
    "lora": ["detail_tweaker"],
    "lora_strength": [0.75]
  },
  "parts": [
    {
      "id": "0194b7a2-8c20-7c11-a801-0a2b3c4d5e6f",
      "role": "metadata",
      "name": "forest_study.xmp",
      "ext": "xmp",
      "size": 4120,
      "status": "Available"
    }
  ]
}
```

## Page {#page}

Paginated list envelope returned by `item.query`, `folder.items` and `smartFolder.items`:

```typescript
interface Page<T> {
  total: number;
  offset: number;
  items: T[];
}
```

| Property | Type | Description |
|---|---|---|
| `total` | `number` | Total number of matching items across all pages. |
| `offset` | `number` | Zero-based starting offset of this slice. |
| `items` | `T[]` | Slice of up to `limit` items (default 50, max 1,000). |

## Added {#added}

Result returned by [`item.add.fromUrls`](../operations/item-add.md#item-add-fromurls), [`item.add.fromBytes`](../operations/item-add.md#item-add-frombytes), [`item.add.bookmark`](../operations/item-add.md#item-add-bookmark), [`item.add.beside`](../operations/item-add.md#item-add-beside) and [`archive.create`](../operations/archive.md#archive-create):

```typescript
type Added =
  | {
      success: true;
      ids: string[];
      errors: string[];
      handed?: { url: string; plugin: string; handler: string }[];
    }
  | {
      success: true;
      queued: true;
    };
```

- **`ids`**: Stable UUIDs of created items.
- **`errors`**: Per-item error strings for URLs or files that failed in a multi-item batch.
- **`handed`**: Present when an enabled plugin URL handler claimed one or more URLs.
- **`queued`**: Set to `true` when `wait: false` or when processing exceeded the synchronous wait timeout. Follow [`items.added`](../events/items.md#items-added) with your `requestId`.
- **`duplicates`** *(runtime only)*: Returned by `item.add.fromUrls` and `item.add.fromBytes` when `onDuplicate` is `skip` or `tell` and matches exist. Entries use `{ id, of }` when kept or `{ of, skipped: true }` when skipped; not declared in `curio.d.ts`.
- **`repeat`** *(runtime only)*: Set to `true` on either branch when a repeated `requestId` replays an earlier result within 10 minutes. Not declared in `curio.d.ts`.
