# Actions

Actions (`actions[]`) wire your plugin's windows, tabs and native programs into Curio's item context menu, toolbar, viewer, keyboard shortcuts and user-built reusable actions.

See [Action and URL handler declarations](../manifest/actions.md) for the complete key reference and defaults.

## Identity

```json
{
  "actions": [
    {
      "id": "convert",
      "title": "Convert pictures…",
      "icon": "transform",
      "where": ["item-menu", "toolbar"],
      "key": "Ctrl+Shift+E",
      "selection": "some",
      "kinds": ["image"],
      "opens": "convert"
    }
  ]
}
```

Each action declares an `id`, `title`, optional Tabler `icon` and optional default shortcut `key`. If `key` collides with an existing Curio shortcut, it starts unassigned until the user binds it in **Settings › Shortcuts**.

## Placement

Set [`where`](../manifest/actions.md#actions-where) to choose where the action appears (defaults to `["item-menu"]`): `"item-menu"` for the right-click menu on matching items, `"toolbar"` for a placeable button in **Edit layout**, `"viewer"` for controls beside an open matching item and `"step"` for Curio's [Build reusable actions](../../features/organize/actions.md). Only program actions (`run`) can appear in `"step"`.

## Conditions

Restrict when an action is active with [`selection`](../manifest/actions.md#actions-selection) (`"some"` default for 1 or more items, `"one"` for 1 item, `"many"` for 2 or more items or `"any"` even with nothing selected), `kinds` (every selected item must match one of the listed kinds) and `exts` (lowercase extensions without dots).

## Page actions

Set **`opens`** to launch one of your plugin's UI surfaces:

- **Standalone window**: `"opens": "<windowId>"` opens a declared `windows[]` page.
- **Workspace tab**: `"opens": "tab:<tabId>"` switches the main workspace to a declared `tabs[]` page.

The opened page receives `{ action: "<id>", items: ["<uuid>", ...] }` via [`plugin.run`](../hooks/lifecycle.md#plugin-run).

## Program actions

Set **`run`** to an argv array of executable and arguments (requires `exec`; never executed through a shell):

```json
{
  "id": "tag-shape",
  "title": "Tag by shape",
  "icon": "dimensions",
  "where": ["item-menu", "step"],
  "kinds": ["image"],
  "run": ["python", "shapes.py"]
}
```

Curio passes the selected items as JSON on `stdin`, provides `CURIO_API_URL` and `CURIO_API_TOKEN` for API access and applies optional `{ message, set }` JSON output from `stdout`. See [Action and URL program contracts](../programs/action-url.md).

## Shortcuts

- Users can rebind or clear any plugin action shortcut under its plugin group in **Settings › Shortcuts**. Plugin pages receive updated bindings live via [`plugin.keys`](../hooks/lifecycle.md#plugin-keys).
- When an action uses `run` and the user has not granted `exec`, Curio shows it disabled with the reason *"needs the 'exec' permission"*.
- Plugin action shortcuts trigger only while the item grid has focus, not while editing text or navigating the Library sidebar.
