// What a drop inside the window does: items onto a folder, a tag, the Trash or a plugin's page, and folders onto folders; drops.js words it while dragging.
import { S, folderOpen } from './state.js'
import { edit, remember } from '../library/edits.js'
import { moveFolders } from '../sidebar/sidebar.js'
import { movesIn } from './drops.js'
import { onDrop, onDragHover } from './dragDrop.js'
import { dropOnFrame } from '../plugins/bridge.js'

function siblingsOf(id) {
  const f = S.sidebar.folders.find((x) => x.id === id)
  return S.sidebar.folders.filter((x) => (x.parent_id ?? null) === (f?.parent_id ?? null))
}
function handleDrop(target, payload, el, zone, mods) {
  const [kind, idStr] = target.split(':')
  const id = Number(idStr)
  if (payload.drop) return payload.drop(target, zone)
  if (payload.type === 'items') {
    const ids = payload.ids
    if (kind === 'frame') dropOnFrame(id, ids, mods)
    else if (kind === 'folder') {
      remember('folders', id)
      if (payload.trashed) edit.restoreTo(ids, id)
      else if (mods?.ctrl) edit.copyTo(ids, id)
      else if (!mods?.shift && movesIn(S.sidebar, payload.root)) edit.moveIn(ids, id)
      else if (payload.from != null && payload.from !== id && !mods?.shift) edit.folder('move_items', { id, ids, value: String(payload.from) })
      else edit.folder('add', { id, ids })
    } else if (kind === 'tag') edit.addTags(ids, [S.sidebar.tags.find((t) => t.id === id)?.name].filter(Boolean))
    else if (kind === 'trash') edit.trash(ids, true)
  } else if (payload.type === 'folder') {
    if (kind === 'folders-root') {
      moveFolders(payload.ids, null)
    } else if (kind === 'folder') {
      if (zone === 'inside') {
        moveFolders(payload.ids.filter((f) => f !== id), id)
        folderOpen[id] = true
      } else {
        const t = S.sidebar.folders.find((x) => x.id === id)
        const sibs = siblingsOf(id).filter((x) => !payload.ids.includes(x.id))
        const at = sibs.findIndex((x) => x.id === id)
        const before = zone === 'before' ? id : sibs[at + 1]?.id ?? null
        moveFolders(payload.ids, t?.parent_id ?? null, before)
      }
    }
  }
}
// a closed folder opens while a drag rests on it
function handleHover(target) {
  if (target.startsWith('folder:')) folderOpen[Number(target.split(':')[1])] = true
}

export function installDrops() {
  onDrop(handleDrop)
  onDragHover(handleHover)
}
