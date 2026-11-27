<script setup>
import { t } from '../shared/i18n.js'
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, shallowRef, watch } from 'vue'
import { S } from '../app/state.js'
import { edit, remember, recentOf } from '../library/edits.js'
import { saveUi } from '../app/preferences.js'
import { loadDetail } from '../library/selection.js'
import { fuzzy, fuzzyRank } from '../shared/text.js'
import { rem } from '../shared/rem.js'
import { call } from '../platform/ipc.js'
import { pickerWidth } from './tags.js'
import AppIcon from '../shared/AppIcon.vue'
import AppHighlight from '../shared/AppHighlight.vue'
import Checkbox from 'openvue/checkbox'
import VirtualScroller from 'openvue/virtualscroller'

// The popups for adding tags and folders.
// S.picker = { kind: 'tags' | 'folders', ids, x, y, above?, value?, onToggle?, byId?, onClose? }:
//   `above`, when set, is where its bottom edge goes instead of its top at `y`, and `end` where its right edge goes instead of its left at `x`;
//   with `ids` a click adds to or removes from those items at once;
//   with `value`/`onToggle` it picks values for a filter (nothing is created): tag names, and folder names or, with `byId`, numbers.
//   `onClose` runs once it closes or another takes its place.
const P = computed(() => S.picker)
const isTags = computed(() => P.value?.kind === 'tags')
const forFilter = computed(() => !!P.value?.onToggle)
const q = ref('')
const input = ref()
const root = ref()
const listEl = ref()
const active = ref(0)
// the active row shows only once the keyboard, the search or the pointer has moved it, so nothing looks chosen on opening
const moved = ref(false)
const tab = ref('all') // folders: all, recent, selected
const group = ref(null) // tags: null = all, 'none' = ungrouped, or a group id
const focusSide = ref(false)
const closed = reactive({})
const collapsedSections = reactive({})
const collapsed = reactive({})
const checked = reactive(new Set())
const excluded = reactive(new Set())
const cfg = computed(() => S.ui.picker)
const NEW_FOLDER_WAIT_MS = 500
// what orders the rows, as it was when the picker opened: a pick moves nothing until it opens again
const atOpen = shallowRef({ recentTags: [], recentFolders: [], checked: new Set(), excluded: new Set(), counts: new Map() })
const freeze = () => (atOpen.value = { recentTags: [...recentOf('tags')], recentFolders: [...recentOf('folders')], checked: new Set(checked), excluded: new Set(excluded), counts: new Map(S.sidebar.tags.map((t) => [t.id, t.count])) })
const wasChecked = (v) => checked.has(v) || atOpen.value.checked.has(v)

// what the selected items already have
async function loadChecked() {
  checked.clear()
  excluded.clear()
  const p = P.value
  if (!p) return
  if (p.value) {
    for (const v of p.value) checked.add(v)
    for (const v of p.excluded || []) excluded.add(v)
    return
  }
  if (!p.ids?.length) return
  const s = p.ids.length === 1 ? await call('asset_detail', { id: p.ids[0] }).catch(() => null) : await call('selection_summary', { ids: p.ids }).catch(() => null)
  const list = (isTags.value ? s?.tags : s?.folders) || []
  for (const x of list) checked.add(isTags.value ? x.name : x.id)
}

// a dialog opened while a popup is up stacks above that popup's layer, so the picker goes above whichever dialog is open
const zIndex = ref(2500)
const aboveDialogs = () => Math.max(2500, ...[...document.querySelectorAll('.p-dialog-mask')].map((m) => (parseInt(m.style.zIndex, 10) || 0) + 1))

watch(P, async (p, old) => {
  if (old && old !== p) old.onClose?.()
  if (!p) return
  zIndex.value = aboveDialogs()
  q.value = ''
  active.value = 0
  moved.value = false
  tab.value = 'all'
  group.value = null
  focusSide.value = false
  rule.value = p.rule || 'OR'
  untaggedOn.value = p.untagged ?? null
  if (p.value) {
    checked.clear()
    excluded.clear()
    for (const v of p.value) checked.add(v)
    for (const v of p.excluded || []) excluded.add(v)
    freeze()
  } else {
    freeze()
    await loadChecked()
    freeze()
  }
  await nextTick()
  resetScroll()
  // asked from a plugin's page, the keyboard is still in its frame
  if (document.activeElement?.tagName === 'IFRAME') document.activeElement.blur()
  input.value?.focus()
}, { immediate: true })

// ---------------------------------------------------------------- tags
const tagGroups = computed(() => S.sidebar.tagGroups)
const groupById = computed(() => new Map(tagGroups.value.map((g) => [g.id, g])))
const groupCounts = computed(() => {
  const byGroup = new Map()
  let ungrouped = 0
  for (const tag of S.sidebar.tags) {
    if (!tag.group_ids?.length) ungrouped++
    else for (const gid of tag.group_ids) byGroup.set(gid, (byGroup.get(gid) || 0) + 1)
  }
  return { all: S.sidebar.tags.length, ungrouped, byGroup }
})
const groupCount = (id) => (id === 'none' ? groupCounts.value.ungrouped : id == null ? groupCounts.value.all : groupCounts.value.byGroup.get(id) || 0)
const sideRows = computed(() => [
  { id: 'selected', name: t('tags.picker.selected') },
  { id: null, name: t('tags.picker.allTags') },
  ...tagGroups.value.map((g) => ({ id: g.id, name: g.name, color: g.color, groupIcon: g.icon || 'bookmark' })),
  { id: 'none', name: t('tags.picker.uncategorized'), icon: 'question-mark' },
])
const sideCount = (g) => (g.id === 'selected' ? checked.size + excluded.size : groupCount(g.id))
const tagsInGroup = computed(() => {
  if (group.value == null || group.value === 'selected') return S.sidebar.tags
  if (group.value === 'none') return S.sidebar.tags.filter((t) => !t.group_ids?.length)
  return S.sidebar.tags.filter((t) => t.group_ids?.includes(group.value))
})
// Starred, Recent, then every group; each folds away.
const tagSections = computed(() => {
  const term = q.value.trim()
  const pool = tagsInGroup.value
  const row = (t, section) => ({ type: 'tag', key: `${section}:${t.id}`, name: t.name, id: t.id, count: t.count, star: t.starred, section })
  const groupNamed = (t) => t.group_ids?.some((g) => fuzzy(groupById.value.get(g)?.name || '', term))
  if (term) {
    const countOf = (t) => atOpen.value.counts.get(t.id) ?? t.count
    const hits = pool.filter((t) => fuzzy(t.name, term) || groupNamed(t)).sort((a, b) => fuzzyRank(a.name, term) - fuzzyRank(b.name, term) || countOf(b) - countOf(a))
    return [{ id: 'results', name: null, rows: hits.map((t) => row(t, 'r')) }]
  }
  if (group.value === 'selected') {
    return [{ id: 'selected', name: null, rows: pool.filter((t) => wasChecked(t.name) || excluded.has(t.name) || atOpen.value.excluded.has(t.name)).map((t) => row(t, 'l')) }]
  }
  const used = new Set()
  const take = (list) => list.filter((t) => !used.has(t.id) && used.add(t.id))
  const out = []
  if (cfg.value.starred) {
    const l = take(pool.filter((t) => t.starred))
    if (l.length) out.push({ id: 'starred', name: t('tags.picker.starred'), rows: l.map((t) => row(t, 's')) })
  }
  if (cfg.value.recent) {
    const byName = new Map(pool.map((t) => [t.name, t]))
    const l = take(atOpen.value.recentTags.map((n) => byName.get(n)).filter(Boolean).slice(0, 12))
    if (l.length) out.push({ id: 'recent', name: t('tags.picker.recent'), rows: l.map((t) => row(t, 'c')) })
  }
  // bucket remaining tags by group in one linear pass rather than scanning the full tag list per group
  const byFirst = new Map()
  for (const tag of pool) {
    if (used.has(tag.id)) continue
    const gid = group.value != null ? (tag.group_ids?.includes(group.value) ? group.value : null) : tag.group_ids?.[0]
    if (gid == null) continue
    if (!byFirst.has(gid)) byFirst.set(gid, [])
    byFirst.get(gid).push(tag)
  }
  for (const g of tagGroups.value) {
    if (group.value != null && group.value !== g.id) continue
    const l = take(byFirst.get(g.id) || [])
    // the tab on the left already names the group it shows
    if (l.length) out.push({ id: 'g' + g.id, name: group.value === g.id ? null : g.name, color: g.color, rows: l.map((t) => row(t, 'g')) })
  }
  const rest = take(pool)
  const extra = forFilter.value && P.value?.onUntagged && group.value == null ? [{ type: 'untagged', key: 'untagged', name: t('tags.picker.untagged'), id: null, count: S.sidebar.counts?.untagged, section: 'o' }] : []
  if (rest.length || extra.length) out.push({ id: 'others', name: out.length && group.value !== 'none' ? (tagGroups.value.length ? t('tags.picker.uncategorized') : t('tags.picker.others')) : null, rows: [...extra, ...rest.map((t) => row(t, 'o'))] })
  return out
})
// a right click on a section's name opens or closes them all
function toggleAllSections(e) {
  e.preventDefault()
  const ids = tagSections.value.map((x) => x.id)
  const close = !ids.some((id) => collapsedSections[id])
  for (const id of ids) collapsedSections[id] = close
}
const RULE_OPTS = computed(() => [['OR', 'layers-union'], ['AND', 'layers-intersect'], ['EQUAL', 'equal']].map(([value, icon]) => ({ value, icon, label: t(`tags.picker.rules.${value}`) })))
const FOLDER_TABS = computed(() => [['all', 'list-details'], ['recent', 'clock'], ['selected', 'copy-check']].map(([value, icon]) => ({ value, icon, label: t(`tags.picker.tabs.${value}`) })))
const TAG_TOGGLES = computed(() => [['sidebar', 'layout-sidebar'], ['starred', 'star'], ['recent', 'clock']].map(([key, icon]) => ({ key, icon, label: t(`tags.picker.toggles.${key}`) })))
const rule = ref('OR')
function setRule(v) {
  rule.value = v
  P.value?.onRule?.(v)
}
// Untagged in a filter: true finds items without tags, false (left out) those with some, null neither
const untaggedOn = ref(null)

// ---------------------------------------------------------------- folders
const folders = computed(() => S.sidebar.folders)
const childrenOf = computed(() => {
  const m = new Map()
  for (const f of folders.value) {
    if (!m.has(f.parent_id ?? 0)) m.set(f.parent_id ?? 0, [])
    m.get(f.parent_id ?? 0).push(f)
  }
  return m
})
const folderById = computed(() => new Map(folders.value.map((f) => [f.id, f])))
const parentName = (f) => folderById.value.get(f.parent_id)?.name || ''
function treeRows() {
  const out = []
  const walk = (parent, depth) => {
    for (const f of childrenOf.value.get(parent) || []) {
      const kids = (childrenOf.value.get(f.id) || []).length > 0
      out.push({ type: 'folder', key: 'f' + f.id, id: f.id, name: f.name, depth, icon: f.icon, color: f.color, parent: parentName(f), kids })
      if (kids && !collapsed[f.id]) walk(f.id, depth + 1)
    }
  }
  walk(0, 0)
  return out
}
const folderRows = computed(() => {
  const term = q.value.trim()
  const plain = (f, extra = {}) => ({ type: 'folder', key: (extra.recent ? 'r' : 'f') + f.id, id: f.id, name: f.name, depth: 0, icon: f.icon, color: f.color, parent: parentName(f), ...extra })
  if (term) {
    return folders.value.filter((f) => fuzzy(f.name, term)).sort((a, b) => fuzzyRank(a.name, term) - fuzzyRank(b.name, term)).map((f) => plain(f))
  }
  const byId = new Map(folders.value.map((f) => [f.id, f]))
  if (tab.value === 'selected') {
    const selected = forFilter.value && !P.value?.byId ? folders.value.filter((f) => wasChecked(f.name)) : [...new Set([...atOpen.value.checked, ...checked])].map((id) => byId.get(id)).filter(Boolean)
    return selected.map((f) => plain(f))
  }
  const recent = atOpen.value.recentFolders.map((id) => byId.get(id)).filter(Boolean)
  if (tab.value === 'recent') return recent.map((f) => plain(f, { recent: true }))
  const tree = treeRows()
  const top = recent.slice(0, 6).map((f) => plain(f, { recent: true }))
  return top.length ? [...top, { type: 'sep', key: 'sep' }, ...tree] : tree
})

// ---------------------------------------------------------------- one flat list for the keyboard
// a filter or a folder to move to picks among what exists, unless it may make a folder
const canCreate = computed(() => !((forFilter.value || P.value?.onPick) && !P.value?.canCreate))
const createRow = computed(() => {
  const term = q.value.trim()
  if (!term || !canCreate.value) return null
  const list = isTags.value ? S.sidebar.tags : folders.value
  if (list.some((x) => x.name.toLowerCase() === term.toLowerCase())) return null
  return { type: 'create', key: 'create', name: term }
})
const rowPx = computed(() => Math.round(rem(2.17)))
const displayLayout = computed(() => {
  void S.ui.fontSize
  void S.ui.density
  const rowH = rowPx.value
  const gap = Math.round(rem(0.14))
  const groupH = Math.round(rem(1.82))
  const sepH = Math.max(7, Math.round(rem(0.56) * (S.ui.density || 1)) + 1)
  const items = []
  const selectable = []
  const rowPos = new Map()
  let y = 0
  const pushRow = (kind, r) => {
    if (items.length && items[items.length - 1].selectable) {
      items[items.length - 1].mb = gap
      items[items.length - 1].h += gap
      y += gap
    }
    const idx = selectable.length
    selectable.push(r)
    const entry = { kind, key: r.key, row: r, y, rowH, mb: 0, h: rowH, idx, selectable: true }
    rowPos.set(idx, entry)
    items.push(entry)
    y += rowH
  }
  if (createRow.value) pushRow('create', createRow.value)
  if (isTags.value) {
    const secs = tagSections.value
    for (let si = 0; si < secs.length; si++) {
      const s = secs[si]
      if (si > 0) {
        items.push({ kind: 'sep', key: 'sep:' + s.id, y, h: sepH, selectable: false })
        y += sepH
      }
      if (s.name) {
        items.push({ kind: 'group', key: 'grp:' + s.id, id: s.id, name: s.name, color: s.color, count: s.rows.length, y, h: groupH, selectable: false })
        y += groupH
      }
      if (!collapsedSections[s.id]) {
        for (const r of s.rows) pushRow('tag', r)
      }
    }
  } else {
    for (const r of folderRows.value) {
      if (r.type === 'sep') {
        items.push({ kind: 'sep', key: r.key, y, h: sepH, selectable: false })
        y += sepH
      } else pushRow('folder', r)
    }
  }
  return { items, selectable, rowPos, total: y }
})
const rows = computed(() => displayLayout.value.selectable)
const itemSizeOf = (i) => displayLayout.value.items[i]?.h ?? rowPx.value
function resetScroll() {
  const vs = listEl.value
  if (!vs) return
  // VirtualScroller's init() leaves `first` where it was when items shrink, so reset it explicitly
  vs.first = 0
  vs.lastScrollPos = 0
  vs.scrollTo({ top: 0 })
  vs.init()
}
watch(displayLayout, () => {
  nextTick(() => {
    const vs = listEl.value
    const el = vs?.$el
    if (!el) return
    if (vs.first >= displayLayout.value.items.length) resetScroll()
    else vs.onScrollChange({ target: el })
  })
})
// focus the first matching result below the create row; fall back to create when nothing matches
const defaultActive = () => (createRow.value && rows.value.length > 1 ? 1 : 0)
watch(rows, () => {
  if (active.value >= rows.value.length) active.value = Math.max(0, rows.value.length - 1)
})
watch(q, (v) => {
  active.value = defaultActive()
  if (v.trim()) moved.value = true
  resetScroll()
})
const isActive = (idx) => moved.value && active.value === idx
const point = (idx) => {
  active.value = idx
  moved.value = true
}
const canExclude = computed(() => !!P.value?.onExclude)
const isExcluded = (r) => canExclude.value && (r.type === 'untagged' ? untaggedOn.value === false : excluded.has(r.name))
function exclude(e, r) {
  e?.preventDefault()
  e?.stopPropagation()
  if (r.type === 'untagged') {
    untaggedOn.value = untaggedOn.value === false ? null : false
    return P.value.onUntagged(untaggedOn.value)
  }
  const v = r.name
  if (excluded.has(v)) excluded.delete(v)
  else {
    checked.delete(v)
    excluded.add(v)
  }
  P.value.onExclude(v)
}
const isChecked = (r) => (r.type === 'untagged' ? untaggedOn.value === true : r.type === 'tag' ? checked.has(r.name) : forFilter.value && !P.value?.byId ? checked.has(r.name) : checked.has(r.id))

async function toggle(r) {
  const p = P.value
  if (!p) return
  if (r.type === 'untagged') {
    untaggedOn.value = untaggedOn.value === true ? null : true
    p.onUntagged(untaggedOn.value)
    return
  }
  if (p.onPick) {
    if (r.type === 'folder') p.onPick(r)
    else if (r.type === 'create') {
      // made now and chosen as soon as it exists
      const name = r.name
      await edit.folder('create', { name })
      const pickIt = p.onPick
      setTimeout(() => {
        const f = S.sidebar.folders.find((x) => x.name === name)
        if (f) pickIt({ type: 'folder', id: f.id, name })
      }, NEW_FOLDER_WAIT_MS)
    }
    close()
    return
  }
  if (p.onToggle) {
    const v = r.type === 'folder' && p.byId ? r.id : r.name
    if (checked.has(v)) checked.delete(v)
    else {
      checked.add(v)
      excluded.delete(v)
    }
    p.onToggle(v)
    if (r.type === 'create') q.value = ''
    return
  }
  const ids = p.ids
  if (r.type === 'create') {
    const name = r.name
    if (isTags.value) {
      await edit.addTags(ids, [name])
      checked.add(name)
      remember('tags', name)
    } else {
      await edit.folder('create', { name, ids })
      // the new folder arrives with the next sidebar refresh
      setTimeout(() => {
        const f = S.sidebar.folders.find((x) => x.name === name)
        if (f) {
          checked.add(f.id)
          remember('folders', f.id)
        }
      }, 500)
    }
  } else if (r.type === 'tag') {
    if (checked.has(r.name)) {
      const known = r.id ?? S.sidebar.tags.find((t) => t.name === r.name)?.id
      if (known != null) await edit.removeTag(ids, known)
      checked.delete(r.name)
    } else {
      await edit.addTags(ids, [r.name])
      checked.add(r.name)
      remember('tags', r.name)
    }
  } else if (checked.has(r.id)) {
    await edit.folder('unadd', { id: r.id, ids })
    checked.delete(r.id)
  } else {
    await edit.folder('add', { id: r.id, ids })
    checked.add(r.id)
    remember('folders', r.id)
  }
  q.value = ''
  loadDetail()
  input.value?.focus()
}

// ---------------------------------------------------------------- keyboard
const sidebarOn = computed(() => isTags.value && (forFilter.value || cfg.value.sidebar))
function reveal() {
  const vs = listEl.value
  const el = vs?.$el
  const item = displayLayout.value.rowPos.get(active.value)
  if (!el || !item) return
  const pad = Math.round(rem(0.28) * (S.ui.density || 1))
  const top = item.y + pad
  const bot = top + item.rowH
  let next = el.scrollTop
  if (top < el.scrollTop + pad) next = Math.max(0, top - pad)
  else if (bot > el.scrollTop + el.clientHeight - pad) next = bot - el.clientHeight + pad
  if (next !== el.scrollTop) {
    vs.scrollTo({ top: next })
    vs.onScrollChange({ target: el })
  }
}
function moveBy(d) {
  const n = rows.value.length
  if (!n) return
  // the first press shows the row the list starts on
  if (moved.value) active.value = (active.value + d + n) % n
  moved.value = true
  reveal()
}
function moveSide(d) {
  const ids = sideRows.value.map((r) => r.id)
  const i = ids.indexOf(group.value)
  group.value = ids[(i + d + ids.length) % ids.length]
  active.value = defaultActive()
  resetScroll()
}
function key(e) {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    return close()
  }
  if (e.key === 'Tab') {
    e.preventDefault()
    if (isTags.value) {
      if (sidebarOn.value) focusSide.value = !focusSide.value
    } else {
      const t = ['all', 'recent', 'selected']
      tab.value = t[(t.indexOf(tab.value) + (e.shiftKey ? 2 : 1)) % 3]
      active.value = 0
      resetScroll()
    }
    return
  }
  const side = focusSide.value && sidebarOn.value
  if (e.key === 'ArrowDown') (side ? moveSide(1) : moveBy(1))
  else if (e.key === 'ArrowUp') (side ? moveSide(-1) : moveBy(-1))
  else if (e.key === 'Enter') {
    // nothing is chosen until a row shows as active
    const r = moved.value ? rows.value[active.value] : null
    if (r && e.shiftKey) canExclude.value && r.type !== 'create' && exclude(null, r)
    else if (r) toggle(r)
  } else return
  e.preventDefault()
  e.stopPropagation()
}
function close() {
  S.picker = null
}
function outside(e) {
  if (!P.value) return
  if (P.value.anchor?.contains?.(e.target)) return
  if (!e.target.closest?.('.picker') && !e.target.closest?.('.p-select-overlay')) close()
}
// a click on a plugin's page never reaches this document: focus moving into its frame says where it went
function blurred() {
  if (P.value && document.activeElement?.tagName === 'IFRAME') close()
}
onMounted(() => {
  window.addEventListener('pointerdown', outside, true)
  window.addEventListener('blur', blurred)
})
onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', outside, true)
  window.removeEventListener('blur', blurred)
})

// ---------------------------------------------------------------- placement
const pickerMaxH = computed(() => {
  const p = P.value
  if (!p) return 0
  const h = isTags.value ? 520 : 380
  if (p.above != null) return Math.min(h, Math.max(6, Math.min(p.above, window.innerHeight - 6)) - 6)
  return h
})
const listHeight = computed(() => {
  if (isTags.value) return '100%'
  const headH = Math.round(rem(2.66))
  const padY = Math.round(rem(0.28) * (S.ui.density || 1)) * 2
  const maxBody = Math.max(80, pickerMaxH.value - headH - 2)
  return Math.min(displayLayout.value.total + padY, maxBody) + 'px'
})
const style = computed(() => {
  const p = P.value
  if (!p) return {}
  const w = pickerWidth(p.kind, forFilter.value)
  const h = pickerMaxH.value
  const left = Math.max(6, Math.min(p.end != null ? p.end - w : p.x, window.innerWidth - w - 6))
  // the folder list is as tall as its rows, up to the most it takes; the tags keep one size as their groups change
  const size = isTags.value ? 'height' : 'maxHeight'
  if (p.above != null) {
    const bottom = Math.max(6, Math.min(p.above, window.innerHeight - 6))
    return { left: left + 'px', bottom: window.innerHeight - bottom + 'px', width: w + 'px', [size]: h + 'px', zIndex: zIndex.value }
  }
  const top = Math.max(6, Math.min(p.y, window.innerHeight - h - 6))
  return { left: left + 'px', top: top + 'px', width: w + 'px', [size]: h + 'px', zIndex: zIndex.value }
})
function setCfg(k, v) {
  S.ui.picker[k] = v
  saveUi()
}
function toggleStar(e, r) {
  e.preventDefault()
  edit.tag(r.star ? 'unstar' : 'star', { id: r.id })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="P" ref="root" class="picker" :class="{ tags: isTags }" :style="style" @keydown="key" @mouseup="input?.focus()">
      <div class="p-head">
        <AppIcon name="search" :size="17" class="faint" />
        <input ref="input" v-model="q" :placeholder="$t('tags.picker.search')" spellcheck="false" maxlength="128" @input="active = defaultActive(); moved = true" />
        <button v-if="q" class="icon-btn clear" @click="q = ''"><AppIcon name="circle-x-filled" :size="16" /></button>
        <template v-if="isTags && P.onRule">
          <button v-for="o in RULE_OPTS" :key="o.value" class="icon-btn" :class="{ on: rule === o.value }" :aria-pressed="rule === o.value" :aria-label="o.label" v-tooltip.bottom="o.label" @click="setRule(o.value)"><AppIcon :name="o.icon" :size="18" /></button>
        </template>
        <template v-else-if="isTags">
          <button v-for="o in TAG_TOGGLES" :key="o.key" class="icon-btn" :class="{ on: cfg[o.key] }" :aria-pressed="!!cfg[o.key]" :aria-label="o.label" v-tooltip.bottom="o.label" @click="setCfg(o.key, !cfg[o.key])"><AppIcon :name="o.icon" :size="18" /></button>
        </template>
        <template v-else>
          <button v-for="o in FOLDER_TABS" :key="o.value" class="icon-btn" :class="{ on: tab === o.value }" :aria-pressed="tab === o.value" :aria-label="o.label" v-tooltip.bottom="o.label" @click="(tab = o.value), (active = 0), resetScroll()"><AppIcon :name="o.icon" :size="18" /></button>
        </template>
      </div>

      <div class="p-body">
        <!-- tag groups -->
        <div v-if="sidebarOn" class="p-side" :class="{ focused: focusSide }">
          <div v-for="g in sideRows" :key="String(g.id)" class="p-side-row" :class="{ on: group === g.id }" @click="group = g.id; active = defaultActive(); resetScroll()">
            <AppIcon v-if="g.groupIcon" :name="g.groupIcon" :size="15" :style="{ color: g.color || undefined }" />
            <AppIcon v-else-if="g.icon" :name="g.icon" :size="13" class="faint" />
            <span class="ellipsis grow">{{ g.name }}</span>
            <span class="count">{{ sideCount(g) }}</span>
          </div>
        </div>

        <VirtualScroller v-if="displayLayout.items.length" ref="listEl" class="p-list" :items="displayLayout.items" :item-size="rowPx" :get-item-size="itemSizeOf" :scroll-height="listHeight" :tabindex="-1">
          <template #item="{ item: it }">
            <div v-if="it.kind === 'create'" :key="it.key" class="p-row create" :class="{ act: isActive(it.idx) }" :style="{ height: it.rowH + 'px', marginBottom: it.mb ? it.mb + 'px' : undefined }" @pointerenter="point(it.idx)" @click="toggle(it.row)">
              <AppIcon name="plus" :size="17" /><AppIcon :name="isTags ? 'tag' : 'folder'" :size="17" class="faint" />
              <i18n-t keypath="tags.picker.create" tag="span" class="ellipsis" scope="global"><template #name><b>{{ it.row.name }}</b></template></i18n-t>
            </div>
            <div v-else-if="it.kind === 'sep'" :key="it.key" class="p-sep" :style="{ height: it.h + 'px' }"><hr /></div>
            <div v-else-if="it.kind === 'group'" :key="it.key" class="p-group" :style="{ height: it.h + 'px' }" @click="collapsedSections[it.id] = !collapsedSections[it.id]" @contextmenu="toggleAllSections">
              <span v-if="it.color" class="group-dot" :style="{ background: it.color }" />
              <span class="grow section-title">{{ it.name }} ({{ it.count }})</span>
              <AppIcon :name="collapsedSections[it.id] ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="faint" />
            </div>
            <div v-else-if="it.kind === 'tag'" :key="it.key" class="p-row" :class="{ act: isActive(it.idx) }" :style="{ height: it.rowH + 'px', marginBottom: it.mb ? it.mb + 'px' : undefined }" @pointerenter="point(it.idx)" @click="toggle(it.row)" @contextmenu="canExclude ? exclude($event, it.row) : it.row.id != null ? toggleStar($event, it.row) : $event.preventDefault()">
              <span v-if="isExcluded(it.row)" class="xed-box"><AppIcon name="x" :size="13" /></span>
              <Checkbox v-else :modelValue="isChecked(it.row)" binary tabindex="-1" class="p-check" />
              <span v-if="!q" class="ellipsis grow">{{ it.row.name }}</span>
              <span v-else class="ellipsis grow"><AppHighlight :text="it.row.name" :q="q" /></span>
              <span v-if="it.row.count != null" class="count">{{ it.row.count }}</span>
            </div>
            <div v-else :key="it.key" class="p-row" :class="{ act: isActive(it.idx) }" :style="{ height: it.rowH + 'px', marginBottom: it.mb ? it.mb + 'px' : undefined, paddingLeft: 6 + it.row.depth * 22 + 'px' }" @pointerenter="point(it.idx)" @click="toggle(it.row)" @contextmenu="canExclude && exclude($event, it.row)">
              <span v-for="d in it.row.depth" :key="d" class="guide" :style="{ left: 14 + (d - 1) * 22 + 'px' }" />
              <span v-if="isExcluded(it.row)" class="xed-box"><AppIcon name="x" :size="13" /></span>
              <Checkbox v-else-if="!P.onPick" :modelValue="isChecked(it.row)" binary tabindex="-1" class="p-check" />
              <span v-if="P.onPick" class="ficon"><AppIcon :name="it.row.icon || 'folder'" :size="18" :style="{ color: it.row.color || undefined }" /><AppIcon v-if="it.row.recent" name="clock" :size="10" class="badge" /></span>
              <span v-if="!q" class="ellipsis grow">{{ it.row.name }}</span>
              <span v-else class="ellipsis grow"><AppHighlight :text="it.row.name" :q="q" /></span>
              <span v-if="it.row.parent && q" class="parent ellipsis">{{ it.row.parent }}</span>
              <button v-if="it.row.kids && !q" class="icon-btn caret" @click.stop="collapsed[it.row.id] = !collapsed[it.row.id]"><AppIcon :name="collapsed[it.row.id] ? 'chevron-right' : 'chevron-down'" :size="14" /></button>
            </div>
          </template>
        </VirtualScroller>
        <div v-else class="p-list">
          <div v-if="isTags && canCreate && !q" class="p-empty">{{ $t('tags.picker.noTags') }}</div>
          <div v-else-if="!isTags && (tab !== 'all' || (canCreate && !q))" class="p-empty">{{ tab === 'recent' ? $t('tags.picker.noRecentFolders') : tab === 'selected' ? $t('tags.picker.nothingSelected') : $t('tags.picker.noFolders') }}</div>
        </div>
      </div>

    </div>
  </Teleport>
</template>

<style>
.picker { position: fixed; z-index: 2500; display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--float-border); border-radius: calc(8px * var(--r)); font-size: 1rem; color: var(--text); overflow: hidden; }
/* as every list's search draws it (preset.js); the rows as the filters' lists draw theirs (AppListboxTristate.vue), the headings and separators as a menu's (AppMenu.vue) */
.p-head { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.66rem, 1px); flex: none; padding: 0 calc(0.7rem * var(--g)) 0 calc(0.98rem * var(--g)); border-bottom: 1px solid var(--border); }
.p-head > .icon-btn { margin-inline-start: calc(-0.42rem * var(--g)); }
.p-head input { flex: 1; min-width: 0; border: 0; outline: 0; padding: 0; background: transparent; color: var(--text); font: inherit; }
.p-head input::placeholder { color: var(--text-faint); }
.p-head .clear { color: var(--text-muted); }
.p-body { flex: 1; min-height: 0; display: flex; }
.p-side { width: round(13.85rem, 1px); flex: none; padding: calc(0.28rem * var(--g)); border-right: 1px solid var(--border); overflow-y: auto; }
.p-side-row { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.17rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.p-side-row + .p-side-row { margin-top: 0.14rem; }
.p-side-row:hover { background: var(--hover); }
.p-side-row.on { background: var(--active); }
.p-side.focused .p-side-row.on { outline: 1px solid color-mix(in srgb, var(--text) 25%, transparent); }
.p-list { flex: 1; min-width: 0; overflow-x: hidden; overflow-y: auto; }
.p-list .p-virtualscroller-content { width: 100%; padding: calc(0.28rem * var(--g)); }
.p-sep { display: flex; align-items: center; padding: 0 calc(0.28rem * var(--g)); }
.p-sep hr { width: 100%; margin: 0; border: 0; border-top: 1px solid var(--border); }
.p-group { display: flex; align-items: center; height: round(1.82rem, 1px); padding: calc(0.14rem * var(--g)) calc(0.42rem * var(--g)) 0; cursor: default; }
.p-group:hover .section-title { color: var(--text); }
.p-row { position: relative; display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.17rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.p-row.act { background: var(--hover); }
.p-row.create b { color: var(--accent); font-weight: inherit; }
.p-row .count { flex: none; }
.guide { position: absolute; top: -0.14rem; bottom: 0; width: 1px; background: var(--border); }
.p-check { pointer-events: none; flex: none; }
.ficon { position: relative; display: inline-flex; color: var(--text-muted); flex: none; }
.ficon .badge { position: absolute; right: -0.28rem; bottom: -0.21rem; background: var(--surface); border-radius: var(--round); }
.parent { max-width: 34%; color: var(--text-faint); }
.icon-btn.caret { min-width: round(1.54rem, 1px); height: round(1.54rem, 1px); padding: 0; }
.p-empty { padding: calc(1.82rem * var(--g)) calc(0.84rem * var(--g)); text-align: center; color: var(--text-faint); }
.group-dot { width: round(0.56rem, 1px); height: round(0.56rem, 1px); margin-right: calc(0.49rem * var(--g)); border-radius: var(--round); flex: none; }
.xed-box { flex: none; width: round(1.26rem, 1px); height: round(1.26rem, 1px); display: inline-flex; align-items: center; justify-content: center; border-radius: calc(4px * var(--r)); background: var(--danger); color: #fff; }
</style>
