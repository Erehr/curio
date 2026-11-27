// Tag colours, and the tag and folder picker.
import { S } from '../app/state.js'

// ------------------------------------------------------------------ tag colours

/** A tag's colour comes from its first group, and shows wherever the tag does. */
export function tagColor(nameOrTag) {
  const t = typeof nameOrTag === 'string' ? S.sidebar.tags.find((x) => x.name === nameOrTag) : nameOrTag
  if (!t) return null
  const g = S.sidebar.tagGroups.find((x) => x.id === t.group_ids?.[0])
  return g?.color || t.color || null
}
/** Colours for a tag chip. */
export function tagTint(name) {
  const c = tagColor(name)
  return c ? { '--tint': c } : {}
}

// ------------------------------------------------------------------ tag and folder popups

/** How wide the popup is: the Tags one is as narrow as a filter's list without its groups pane. */
export const pickerWidth = (kind, forFilter) => (kind !== 'tags' ? 400 : forFilter || S.ui.picker.sidebar ? 440 : 260)

/** Open the popup beside whatever was clicked; clicking the same trigger again closes it. */
export function openPicker(kind, e, opts = {}) {
  const anchor = e?.currentTarget || null
  if (anchor && S.picker?.anchor === anchor && S.picker?.kind === kind) {
    S.picker = null
    return
  }
  const r = anchor?.getBoundingClientRect?.()
  const w = pickerWidth(kind, !!opts.onToggle)
  let x = (window.innerWidth - w) / 2
  let y = 90
  let above = null
  let end = null
  const zone = e?.currentTarget?.closest?.('.zone.inspector')
  if (zone) {
    // beside the Inspector rather than over it, so its chips show each change
    const z = zone.getBoundingClientRect()
    if (zone.classList.contains('bottom-inspector')) {
      x = r.left
      above = z.top - 6
    } else {
      // left of it, the right edge stays by it as the groups pane comes and goes
      if (z.left + z.width / 2 > window.innerWidth / 2) end = z.left - 6
      else x = z.right + 6
      y = r.top - 8
    }
  } else if (r) {
    const onRight = r.left > window.innerWidth / 2
    if (onRight) end = r.left - 12
    else x = r.left
    y = onRight ? r.top - 56 : r.bottom + 6
  } else if (e?.clientX != null) {
    // from a context menu: where it was opened
    x = e.clientX
    y = e.clientY
  } else {
    // from a shortcut or a menu: below the selected item
    const cell = document.querySelector(`[data-item="${S.selected[S.selected.length - 1]}"]`)?.getBoundingClientRect()
    if (cell) {
      x = cell.left
      y = cell.bottom + 6
    }
  }
  S.picker = { ...opts, kind, anchor, ids: opts.ids || [...S.selected], x, y, above, end }
}
