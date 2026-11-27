// Edits to items, folders and tags through the Rust core, with the recently used and favourites.
import { call, errorText } from '../platform/ipc.js'
import { S } from '../app/state.js'
import { t } from '../shared/i18n.js'
import { run, toast, playNotifySound } from '../app/notify.js'
import { requestSearch } from './results.js'
import { removeFromView } from './navigation.js'
import { saveUi } from '../app/preferences.js'
import { L } from '../layout/arrangement.js'

/** Reads these items' files again, with every reader or those filling `keys`; what was set by hand stays. */
export async function readMetadataAgain(ids, keys) {
  const r = await run('field.rescan', { items: ids, mode: 'all', ...(keys ? { keys } : {}) }).catch(() => null)
  if (r) toast(r.files ? t('library.rescanning', { count: r.files }) : t('library.nothingToRescan'))
}

// ------------------------------------------------------------------ edits

const TAG_RENAMED_MS = 2500
const folderName = (id) => S.sidebar.folders.find((f) => f.id === id)?.name || t('library.views.folder')
const tagName = (id) => S.sidebar.tags.find((t) => t.id === id)?.name || t('library.views.tag')

export const edit = {
  rating: (ids, rating) => {
    for (const id of ids) {
      const it = S.items.get(id)
      if (it) it.rating = rating
    }
    S.itemsVer++
    return run('item.update', { ids, rating })
  },
  favorite: (ids, favorite) => {
    for (const id of ids) {
      const it = S.items.get(id)
      if (it) it.favorite = favorite
    }
    S.itemsVer++
    return run('item.update', { ids, favorite })
  },
  explicit: (ids, explicit) => {
    for (const id of ids) {
      const it = S.items.get(id)
      if (it) it.explicit = explicit
    }
    S.itemsVer++
    return run('item.update', { ids, explicit })
  },
  addTags: (ids, names) => run('item.addTags', { ids, tags: names }),
  removeTag: (ids, tagId) => run('item.removeTags', { ids, tags: [tagId] }),
  clearTags: (ids) => run('item.removeTags', { ids, all: true }),
  /** Tag names kept for pasting onto other items in this window; also put on the clipboard as text. */
  copyTagNames: (names) => {
    S.copiedTags = [...new Set(names)]
    navigator.clipboard?.writeText(S.copiedTags.join(', ')).catch(() => {})
    toast(S.copiedTags.length === 1 ? t('library.copiedTag', { name: S.copiedTags[0] }) : t('library.copiedTags', { count: S.copiedTags.length }))
  },
  pasteTags: (ids) => (S.copiedTags?.length && ids.length ? edit.addTags(ids, S.copiedTags) : Promise.resolve()),
  /** `what` is name, annotation or url. */
  text: (ids, what, value) => run('item.update', { ids, [what]: value }),
  field: (ids, key, value) => run('item.update', { ids, fields: { [key]: value } }),
  defineField: ({ key, label, type }) => run('field.define', { key, label, type }),
  /** In a folder view, Delete trashes the items' entries in that folder; they stay in their other folders. */
  trash: (ids, deleted) => {
    if (!ids.length) return
    const folder = deleted && S.scope.kind === 'folder' ? S.scope.id : null
    const go = (d) => (folder != null ? run('trash_in_folder', { ids, folder, deleted: d }) : run(d ? 'item.trash' : 'item.restore', { ids }))
    go(deleted)
    removeFromView(ids)
    const undo = async () => {
      await go(!deleted)
      requestSearch()
    }
    toast(deleted ? t('library.trashed', { count: ids.length }) : t('library.restored', { count: ids.length }), 'info', undefined, undo, deleted ? 'deleteItem' : 'other')
  },
  purge: (ids) => {
    if (!ids.length) return
    S.dialogs.confirm = {
      title: t('library.purge.title', { count: ids.length }),
      body: t('library.purge.body'),
      okLabel: t('library.purge.ok'),
      ok: async () => {
        await run('item.purge', { ids })
        removeFromView(ids)
        playNotifySound('deleteItem')
      },
    }
  },
  folder: async (op, args) => {
    const name = args.id != null ? folderName(args.id) : ''
    const r = await folderOp(op, args)
    const n = args.ids?.length || 0
    const again = (other) => () => folderOp(other, args)
    if (op === 'add' && n) toast(t('library.folders.added', { count: n, name }), 'info', undefined, again('unadd'))
    // taken out of a folder, an item stays in the library, at its top level when it was in no other
    else if (op === 'unadd' && n) toast(t('library.folders.takenOut', { count: n, name }), 'info', undefined, again('add'))
    else if (op === 'move_items' && n) toast(t('library.folders.moved', { count: n, name }))
    else if (op === 'delete') toast(t('library.folders.removed', { name }), 'info', undefined, null, 'deleteFolder')
    else if (op === 'rename' && args.name && args.name !== name) toast(t('library.renamed', { name, to: args.name }))
    return r
  },
  /** `look` is { icon } or { color }; null clears it. */
  locationLook: (id, look) => shownAtOnce('roots', setRow(id, look), () => run('location.setLook', { id, ...Object.fromEntries(Object.entries(look).map(([k, v]) => [k, v ?? ''])) })),
  /** Items dragged out of the trash onto a folder come back in it alone. */
  restoreTo: async (ids, folder) => {
    if (!ids.length) return
    await run('restore_to_folder', { ids, folder })
    removeFromView(ids)
    toast(t('library.restoredTo', { count: ids.length, name: folderName(folder) }))
  },
  /** Copy items to another library; a move then puts them in this library's trash. */
  sendTo: async (lib, ids, move) => {
    if (!ids.length) return
    try {
      const { count: n } = await call(move ? 'item.moveToLibrary' : 'item.copyToLibrary', { ids, to: lib.path })
      if (move) removeFromView(ids)
      toast(move ? t('library.movedToLibrary', { name: lib.name }) : t('library.copiedToLibrary', { name: lib.name }))
    } catch (e) {
      toast(errorText(e), 'error')
    }
  },
  /** Move linked items' files into the library after a confirmation; with `folder` they join it and their files land there. */
  moveIn: (ids, folder = null) => {
    if (!ids.length) return
    S.dialogs.confirm = {
      title: t('library.moveIn.title', { count: ids.length }),
      body: folder != null ? t('library.moveIn.intoFolder', { name: folderName(folder) }) : t('library.moveIn.intoLibrary'),
      okLabel: t('library.moveIn.ok'),
      okSeverity: 'primary',
      ok: () => run('transfer', { action: 'move_in', ids, folder }),
    }
  },
  /** Independent copies of the items in `folder`: new items with their own bytes. */
  copyTo: async (ids, folder) => {
    if (!ids.length) return
    await run('copy_to_folder', { ids, folder })
    toast(t('library.copiedTo', { count: ids.length, name: folderName(folder) }))
  },
  tag: async (op, args) => {
    const name = args.id != null ? tagName(args.id) : ''
    const r = await tagOp(op, args)
    if (op === 'rename' && args.name && args.name !== name) toast(t('library.renamed', { name, to: args.name }), 'info', TAG_RENAMED_MS)
    return r
  },
  smart: (op, a) => {
    if (op === 'create') return run('smartFolder.create', { name: a.name ?? t('library.newSmartName'), query: a.query ?? '', parentId: a.parent ?? null, groupBy: a.groupBy ?? null })
    if (op === 'update') return shownAtOnce('smart', setRow(a.id, Object.fromEntries(Object.entries({ name: a.name, query: a.query, group_by: a.groupBy === undefined ? undefined : a.groupBy || null }).filter(([, v]) => v !== undefined))), () => run('smartFolder.update', { id: a.id, name: a.name ?? null, query: a.query ?? null, groupBy: a.groupBy ?? null }))
    if (op === 'delete') return run('smartFolder.delete', { id: a.id })
    if (op === 'place') return shownAtOnce('smart', placeRow(a.id, a.parent ?? null, a.before ?? null), () => run('smart_op', { op, id: null, name: null, query: null, icon: null, ...a }))
    if (op === 'icon') return shownAtOnce('smart', setRow(a.id, { icon: a.icon || null }), () => run('smartFolder.update', { id: a.id, icon: a.icon ?? '' }))
    return run('smart_op', { op, id: null, name: null, query: null, icon: null, ...a })
  },
}

// The sidebar shows a change to its folders, locations, tags and smart folders at once; the sidebar read after the change confirms it, and a change refused is put back.
const nameOrder = (a, b) => (a.name.toLowerCase() < b.name.toLowerCase() ? -1 : a.name.toLowerCase() > b.name.toLowerCase() ? 1 : 0)
const placeOrder = (a, b) => a.sort_order - b.sort_order || nameOrder(a, b)
// as Db::folders and Db::smart_folders list them
const ORDER = { folders: placeOrder, smart: placeOrder }
async function shownAtOnce(list, change, request) {
  const rows = S.sidebar[list]
  const before = rows.map((r) => ({ ...r }))
  change(rows)
  if (ORDER[list]) rows.sort(ORDER[list])
  try {
    return await request()
  } catch (e) {
    if (S.sidebar[list] === rows) S.sidebar[list] = before
    throw e
  }
}
const setRow = (id, values) => (rows) => Object.assign(rows.find((r) => r.id === id) || {}, values)
// Db::place_folder and place_smart_folder: numbered again among its new siblings, before `before` or last
const placeRow = (id, parent, before) => (rows) => {
  const f = rows.find((r) => r.id === id)
  if (!f) return
  f.parent_id = parent
  const sibs = rows.filter((r) => r !== f && (r.parent_id ?? null) === parent).sort(placeOrder)
  const at = sibs.findIndex((r) => r.id === before)
  sibs.splice(at < 0 ? sibs.length : at, 0, f)
  sibs.forEach((r, i) => (r.sort_order = i + 1))
}
// items moved out of the folder on show leave it at once, unless they land in a subfolder it shows too
function leavesView(from, to) {
  if (S.scope.kind !== 'folder' || S.scope.id !== from) return false
  if (!L().subfolders) return true
  for (let p = to; p != null; p = S.sidebar.folders.find((f) => f.id === p)?.parent_id ?? null) if (p === from) return false
  return true
}

// Changes the API has operations for go through them; ordering, pinning, sorting and the like stay the window's own.
async function folderOp(op, a) {
  const into = (ids, id) => ({ ids, folders: [id] })
  switch (op) {
    case 'create': {
      const r = await run('folder.create', { name: a.name ?? t('library.newFolderName'), parent: a.parent ?? null })
      if (a.ids?.length) await run('item.addToFolders', into(a.ids, r.id))
      return r
    }
    case 'rename': return shownAtOnce('folders', setRow(a.id, { name: (a.name || '').trim() || folderName(a.id) }), () => run('folder.rename', { id: a.id, name: a.name }))
    case 'delete': return run('folder.delete', { id: a.id })
    case 'move': return shownAtOnce('folders', setRow(a.id, { parent_id: a.parent ?? null }), () => run('folder.move', { id: a.id, parent: a.parent ?? null }))
    case 'place': return shownAtOnce('folders', placeRow(a.id, a.parent ?? null, a.before ?? null), () => folderOpCall(op, a))
    case 'pin': return shownAtOnce('folders', setRow(a.id, { pinned: a.value === 'true' }), () => folderOpCall(op, a))
    case 'color': return shownAtOnce('folders', setRow(a.id, { color: a.value || null }), () => run('folder.update', { id: a.id, color: a.value ?? '' }))
    case 'icon': return shownAtOnce('folders', setRow(a.id, { icon: a.value || null }), () => run('folder.update', { id: a.id, icon: a.value ?? '' }))
    case 'auto_tags': return run('folder.update', { id: a.id, autoTags: (a.value || '').split(',').map((t) => t.trim()).filter(Boolean) })
    case 'add': return run('item.addToFolders', into(a.ids, a.id))
    case 'unadd': return run('item.removeFromFolders', into(a.ids, a.id))
    case 'move_items': {
      const from = a.value && a.value !== 'all' ? Number(a.value) : null
      if (from != null && leavesView(from, a.id)) removeFromView(a.ids)
      try {
        return await run('item.moveToFolder', { ids: a.ids, folder: a.id, from })
      } catch (e) {
        requestSearch()
        throw e
      }
    }
    default: return folderOpCall(op, a)
  }
}
const folderOpCall = (op, a) => run('folder_op', { op, id: null, ids: null, name: null, parent: null, value: null, before: null, ...a })

async function tagOp(op, a) {
  switch (op) {
    case 'rename': return shownAtOnce('tags', setRow(a.id, { name: (a.name || '').trim() || tagName(a.id) }), () => run('tag.rename', { tag: a.id, newName: a.name }))
    case 'delete': return run('tag.delete', { tag: a.id })
    case 'color': return shownAtOnce('tags', setRow(a.id, { color: a.color || null }), () => run('tag.setColor', { tag: a.id, color: a.color ?? '' }))
    case 'star': case 'unstar': return shownAtOnce('tags', setRow(a.id, { starred: op === 'star' }), () => run('tag_op', { op, id: null, name: null, group: null, color: null, ...a }))
    case 'rename_group': return shownAtOnce('tagGroups', setRow(a.id, { name: a.name }), () => run('tagGroup.update', { id: a.id, name: a.name }))
    case 'group_look': return shownAtOnce('tagGroups', setRow(a.id, { color: a.color || null, icon: a.name || null }), () => run('tagGroup.update', { id: a.id, color: a.color ?? '', icon: a.name ?? '' }))
    case 'group_description': return shownAtOnce('tagGroups', setRow(a.id, { description: a.name || null }), () => run('tagGroup.update', { id: a.id, description: a.name ?? '' }))
    case 'delete_group': return run('tagGroup.delete', { id: a.id })
    case 'new_group': return run('tagGroup.create', { name: a.name ?? t('library.newGroupName'), color: a.color ?? null })
    case 'add_to_group': return run('tagGroup.addTags', { groupId: a.group, tags: [a.id] })
    case 'remove_from_group': return run('tagGroup.removeTags', { groupId: a.group, tags: [a.id] })
    default: return run('tag_op', { op, id: null, name: null, group: null, color: null, ...a })
  }
}

// ------------------------------------------------------------------ recently used (per library)

const RECENT_MAX = 20

function recentStore() {
  const key = S.startup.current || ''
  return S.ui.recents[key] || (S.ui.recents[key] = { tags: [], folders: [] })
}
export const recentOf = (kind) => recentStore()[kind] || []
export function remember(kind, value) {
  const r = recentStore()
  r[kind] = [value, ...(r[kind] || []).filter((x) => x !== value)].slice(0, RECENT_MAX)
  saveUi()
}

// ------------------------------------------------------------------ favorites

export const isFavorite = (kind, id) => (S.sidebar.favorites || []).some((f) => f.kind === kind && f.id === id)
export function toggleFavorite(kind, id) {
  return run('favorite_op', { op: isFavorite(kind, id) ? 'remove' : 'add', kind, id, list: null })
}
export function reorderFavorites(list) {
  S.sidebar.favorites = list
  return run('favorite_op', { op: 'set', kind: null, id: null, list })
}

export function addToLastFolder(ids) {
  const id = recentOf('folders')[0]
  const f = S.sidebar.folders.find((x) => x.id === id)
  if (!f) return toast(t('library.noRecentFolder'), 'error')
  edit.folder('add', { id, ids })
}
