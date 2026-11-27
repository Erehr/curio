# Overlays

Because every plugin page runs inside an iframe, HTML dropdowns and popovers rendered inside the page clip at the iframe's edges, especially in compact widgets. Curio provides host-rendered overlays drawn directly over the Curio window.

## Host dialogs

With the `ui` permission, any plugin page can invoke Curio's native dialogs and pickers in the user's theme and language against the live library:

- **[`ui.notify`](../../api/operations/ui.md#ui-notify)**: Non-blocking toast (`info` or `error`).
- **[`ui.confirm`](../../api/operations/ui.md#ui-confirm) / [`ui.prompt`](../../api/operations/ui.md#ui-prompt)**: Confirmation and single-line text input dialogs.
- **[`ui.pickTags`](../../api/operations/ui.md#ui-picktags) / [`ui.pickFolders`](../../api/operations/ui.md#ui-pickfolders) / [`ui.pickField`](../../api/operations/ui.md#ui-pickfield) / [`ui.pickColor`](../../api/operations/ui.md#ui-pickcolor)**: Native Curio pickers.
- **[`ui.menu`](../../api/operations/ui.md#ui-menu)**: Native context menu anchored at the last pointer position, or beneath a viewer toolbar button when triggered from `plugin.tool` with `menu: true`.
- **[`dialog.openFile`](../../api/operations/dialog.md#dialog-openfile) / [`dialog.openFolder`](../../api/operations/dialog.md#dialog-openfolder) / [`dialog.saveFile`](../../api/operations/dialog.md#dialog-savefile)**: Native OS file and folder choosers.

## ui.choose

[`ui.choose`](../../api/operations/ui.md#ui-choose) requires **no extra permission** (covered by `library.read`) and renders a native Curio dropdown menu anchored under any button or `.curio-select-btn` in your page:

```javascript
const btn = document.querySelector('#status-btn')
btn.addEventListener('click', async () => {
  const r = btn.getBoundingClientRect()
  const res = await curio.ui.choose({
    at: { x: r.left, y: r.top, w: r.width, h: r.height },
    value: currentStatus,
    search: false,
    options: [
      { value: 'unread', label: 'Unread' },
      { value: 'reading', label: 'Reading', icon: 'book' },
      { value: 'finished', label: 'Finished', icon: 'check' },
    ],
  })
  if (res.chosen) await saveStatus(res.value)
})
```

- **Options**: Supports up to **1,000** options (`string | number` or `{ label, value, icon?, hint?, heading?, disabled? }`) plus optional live search (`search: true`).
- **Focus restoration**: After closing, Curio returns keyboard focus to your plugin iframe.

## ui.popover

[`ui.popover`](../../api/operations/ui.md#ui-popover) requires **no extra permission** and opens another HTML page from your plugin folder inside a floating Curio popover anchored to an element in your calling page:

```javascript
const r = triggerBtn.getBoundingClientRect()
const { result } = await curio.ui.popover({
  page: 'picker-popover.html',
  width: 320, // 160 to 640 px (default 300)
  at: { x: r.left, y: r.top, w: r.width, h: r.height },
  data: { itemId: currentItem.id, initialColor: '#3b9160' },
})
if (result) applyChoice(result)
```

Only one popover can be open at a time.

## Popover data

Inside `picker-popover.html`:

```javascript
const curio = Curio.connect()
curio.on('plugin.run', ({ data }) => {
  renderPicker(data.itemId, data.initialColor)
})

applyBtn.addEventListener('click', () => {
  curio.plugin.close({ result: { color: selectedColor } })
})
```

- **Auto-sizing**: `curio.js` observes `document.body` height so the popover resizes to fit its contents (up to 4,000 px).
- **Returning data**: Calling `curio.plugin.close({ result })` closes the popover and resolves the caller's `ui.popover` promise with `{ result }`.

## Dismissal

- **Dismiss triggers**: Clicking outside a `ui.choose` menu or `ui.popover` frame, or pressing ++escape++, dismisses the overlay.
- **Return values**: `ui.choose` resolves to `{ chosen: false }` when dismissed; `ui.popover` resolves to `{ result: null }` when dismissed without calling `plugin.close({ result })`.
- **Focus**: Keyboard focus returns to the calling plugin frame so arrow keys and shortcuts keep working.
