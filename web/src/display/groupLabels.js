// Section heading labels for Group by spans and lazy cover fetching for Subfolders (folders and location directories).
import { markRaw, ref } from 'vue'
import { S, folderOpen, rootOpen } from '../app/state.js'
import { call } from '../platform/ipc.js'
import { thumbUrl } from '../platform/fileUrls.js'
import { setScope } from '../library/navigation.js'
import { directRootDirs } from '../sidebar/sidebar.js'
import { t } from '../shared/i18n.js'

const KNOWN_KINDS = new Set(['image', 'video', 'audio', 'document', 'code', 'link', 'archive', 'font', 'model', 'other'])
const KNOWN_DATES = new Set(['later', 'today', 'yesterday', 'thisWeek', 'lastWeek', 'thisMonth', 'lastMonth', 'thisYear', 'unknown'])
const KNOWN_SIZES = new Set(['empty', 'tiny', 'small', 'medium', 'large', 'huge'])

/** Localized heading text for one layout section. */
export function sectionTitle(sec) {
  switch (sec.kind) {
    case 'contents': return t('grid.groups.contents')
    case 'kind': return KNOWN_KINDS.has(sec.val) ? t(`grid.groups.kinds.${sec.val}`) : sec.val
    case 'ptype': return sec.val
    case 'date': return KNOWN_DATES.has(sec.val) ? t(`grid.groups.dates.${sec.val}`) : sec.val
    case 'size': return KNOWN_SIZES.has(sec.val) ? t(`grid.groups.sizes.${sec.val}`) : sec.val
    case 'rating': {
      const n = Number(sec.val)
      return n > 0 ? t('grid.groups.rating', { count: n.toLocaleString() }, n) : t('grid.groups.unrated')
    }
    case 'location':
      if (sec.val === 'library') return t('grid.groups.library')
      if (sec.val === 'linked') return t('grid.groups.linked')
      return S.sidebar.roots.find((r) => String(r.id) === sec.val)?.label || t('grid.groups.location')
    case 'bool': return sec.val === '1' ? t('filters.value.yes') : t('filters.value.no')
    case 'empty': return t('grid.groups.empty')
    default: return sec.val
  }
}

/** Direct child subfolders of the active scope (Curio folder or indexed Location directory). */
export function scopeSubs() {
  if (S.scope.kind === 'folder') {
    return S.sidebar.folders
      .filter((f) => f.parent_id === S.scope.id)
      .map((f) => ({ key: 'f:' + f.id, kind: 'folder', id: f.id, parentId: f.parent_id, name: f.name, count: f.count, color: f.color, icon: f.icon, createdAt: f.created_at, drop: 'folder:' + f.id }))
  }
  if (S.scope.kind === 'root' || S.scope.kind === 'rootdir') {
    return directRootDirs(S.scope.id, S.scope.kind === 'rootdir' ? S.scope.path || '' : '').map((d) => ({
      key: 'd:' + S.scope.id + ':' + d.path,
      kind: 'rootdir',
      rootId: S.scope.id,
      path: d.path,
      name: d.name,
      count: d.count,
      color: null,
      icon: null,
      createdAt: 0,
      drop: null,
    }))
  }
  return []
}

/** Navigate into a subfolder item produced by `scopeSubs()`. */
export function openSubfolder(sub) {
  if (sub.kind === 'folder') {
    folderOpen[sub.parentId] = true
    setScope('folder', sub.id)
  } else {
    rootOpen[sub.rootId] = true
    if (S.scope.kind === 'rootdir' && S.scope.path) rootOpen[sub.rootId + ':' + S.scope.path] = true
    setScope('rootdir', sub.rootId, null, sub.path)
  }
}

// ------------------------------------------------------------------ lazy subfolder covers

const covers = markRaw(new Map())
export const subCoversVer = ref(0)
let coversLib = null
const pendingFolders = new Map()
const pendingDirs = new Map()
let flushTimer = null

function checkLibrary() {
  if (coversLib === S.startup.current) return
  coversLib = S.startup.current
  covers.clear()
  pendingFolders.clear()
  pendingDirs.clear()
  clearTimeout(flushTimer)
  flushTimer = null
}

/** Drop cached covers that had no thumbnail yet so freshly built thumbnails can populate them. */
export function retryEmptyCovers() {
  let dropped = false
  for (const [k, v] of covers) {
    if (!v.thumbs?.length && v.count > 0) {
      covers.delete(k)
      dropped = true
    }
  }
  if (dropped) subCoversVer.value++
}

export function coverOf(sub) {
  void subCoversVer.value
  checkLibrary()
  return covers.get(sub.key) || null
}

export function coverPic(sub) {
  const thumb = coverOf(sub)?.thumbs?.[0]
  return thumb ? thumbUrl(thumb.uid, thumb.ver) : null
}

export function coverColor(sub) {
  return coverOf(sub)?.color || null
}

export function ensureSubCovers(list) {
  checkLibrary()
  let any = false
  for (const sub of list) {
    const cached = covers.get(sub.key)
    if (cached && cached.stamp === sub.count) continue
    if (sub.kind === 'folder') {
      if (!pendingFolders.has(sub.id)) {
        pendingFolders.set(sub.id, sub)
        any = true
      }
    } else {
      let bucket = pendingDirs.get(sub.rootId)
      if (!bucket) pendingDirs.set(sub.rootId, (bucket = new Map()))
      if (!bucket.has(sub.path)) {
        bucket.set(sub.path, sub)
        any = true
      }
    }
  }
  if (any && !flushTimer) flushTimer = setTimeout(flushCovers, 0)
}

async function flushCovers() {
  flushTimer = null
  const lib = coversLib
  const folders = [...pendingFolders.values()]
  pendingFolders.clear()
  const dirsByRoot = [...pendingDirs.entries()].map(([rootId, map]) => [rootId, [...map.values()]])
  pendingDirs.clear()
  let changed = false
  if (folders.length) {
    const res = await call('folder_covers', { ids: folders.map((f) => f.id) }).catch(() => null)
    if (res && lib === coversLib) {
      for (const f of folders) {
        const hit = res[f.id]
        if (hit) covers.set(f.key, { thumbs: hit.thumbs || [], count: hit.count, color: hit.color || null, stamp: f.count })
      }
      changed = true
    }
  }
  for (const [rootId, subs] of dirsByRoot) {
    const res = await call('root_dir_covers', { root: rootId, paths: subs.map((d) => d.path) }).catch(() => null)
    if (res && lib === coversLib) {
      for (const d of subs) {
        const hit = res[d.path]
        if (hit) covers.set(d.key, { thumbs: hit.thumbs || [], count: hit.count, color: hit.color || null, stamp: d.count })
      }
      changed = true
    }
  }
  if (changed) subCoversVer.value++
}
