// What a drop will do, shown while dragging. The rules are written out in docs/features/organize/drag-drop.md.
import { t } from '../shared/i18n.js'

/** Items dragged inside the window, over the target `kind:id` (or nothing). `from` is the folder being viewed: dragged out of it they move; with `shift` (or from anywhere else) the same item is linked into the folder and stays where it is; with `ctrl` each becomes an independent copy. `root` is the location being viewed: from one set to organize files they move into the library. `trashed`: dragged out of the Trash, they are restored into the folder. */
export function itemDropText(target, n, sidebar, { shift = false, ctrl = false, from = null, root = null, trashed = false } = {}) {
  const [kind, idStr] = (target || '').split(':')
  const id = Number(idStr)
  switch (kind) {
    case 'folder': {
      const name = sidebar?.folders?.find((f) => f.id === id)?.name || t('app.drops.theFolder')
      if (trashed) return t('app.drops.restore', { count: n, name })
      if (ctrl) return t('app.drops.copy', { count: n, name })
      if (!shift && movesIn(sidebar, root)) return t('app.drops.moveIn', { count: n, name })
      const out = from != null && from !== id ? sidebar?.folders?.find((f) => f.id === from)?.name || t('app.drops.thisFolder') : null
      if (!out) return t('app.drops.link', { count: n, name })
      if (shift) return t('app.drops.linkKeeping', { count: n, name, from: out })
      return t('app.drops.move', { count: n, name, from: out })
    }
    case 'tag': {
      const name = sidebar?.tags?.find((t) => t.id === id)?.name
      return t('app.drops.tag', { count: n, name: name || '' }).trim()
    }
    case 'trash':
      return t('app.drops.trash', { count: n })
    case 'frame':
      return t('app.drops.plugin', { count: n })
    default:
      return t('app.drops.nowhere', { count: n })
  }
}

/** Whether items dragged from the location `root` move into the library: its files may be organized. */
export function movesIn(sidebar, root) {
  return root != null && sidebar?.roots?.find((r) => r.id === root)?.access === 'organize'
}

/** Files dragged in from the desktop, which are copied in; `folder` the folder under the pointer or shown, if any. */
export function fileDropText({ folder, library }) {
  if (library) return t('app.drops.openLibrary')
  return folder ? t('app.drops.copyInto', { name: folder }) : t('app.drops.copyIntoLibrary')
}
