<script>
// where the grid was when the viewer replaced it
let kept = null
</script>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, reactive } from 'vue'
import Checkbox from 'openvue/checkbox'
import { badgeShown, L } from '../layout/arrangement.js'
import { filterQuery } from '../filters/query.js'
import { tagTint } from '../tags/tags.js'
import { S } from '../app/state.js'
import { ensureItems, ensureCells } from '../library/results.js'
import { selectOnly, toggleSelect, selectRange, clearSelection, loadDetail, selectAll, selectSpan } from '../library/selection.js'
import { openViewer } from '../viewer/session.js'
import { unreadable } from '../library/navigation.js'
import { toast, run } from '../app/notify.js'
import { openItem, pasteClipboard } from '../library/lifecycle.js'
import { saveUi } from '../app/preferences.js'
import { spacingScale } from '../app/appearance.js'
import { startNewFolder, loadRootDirs } from '../sidebar/sidebar.js'
import { edit } from '../library/edits.js'
import { GRID_GAP, computeFolderSection, visibleFolderIndices, layoutFor, visibleItems, visibleRange, visibleSections, sectionOfIndex } from '../shared/gridLayout.js'
import { call, errorText } from '../platform/ipc.js'
import { originalUrl, textUrl } from '../platform/fileUrls.js'
import { KIND_ICON, formatOff, thumbOf } from '../formats/index.js'
import { beginDrag } from '../app/dragDrop.js'
import { itemDropText } from '../app/drops.js'
import AppIcon from '../shared/AppIcon.vue'
import { rem, remPx } from '../shared/rem.js'
import ItemMenu from '../library/ItemMenu.vue'
import AppMenu from '../shared/AppMenu.vue'
import { shortcutLabel } from '../app/shortcuts.js'
import DisplayGridListHeader from './DisplayGridListHeader.vue'
import DisplayGridSectionHeader from './DisplayGridSectionHeader.vue'
import HoverPlayer from '../formats/HoverPlayer.vue'
import FontSample from '../formats/FontSample.vue'
import { allColumns, fmtDay, fmtDate, listRowHeight } from './listColumns.js'
import { sectionTitle, scopeSubs, openSubfolder, coverOf, coverPic, ensureSubCovers, retryEmptyCovers } from './groupLabels.js'
import { t } from '../shared/i18n.js'

const lay = computed(() => L())
const scroller = ref()
const width = ref(800)
const height = ref(600)
const scrollTop = ref(0)
const menu = ref()
const secMenu = ref()
const band = ref(null)
const PAD = 12

// the extension has a toggle of its own: it ends the name, or stands alone in the line below without one
const bareTitle = (it) => (it.ext && it.title.toLowerCase().endsWith('.' + it.ext.toLowerCase()) ? it.title.slice(0, -it.ext.length - 1) : it.title)
const shownTitle = (it) => (lay.value.showExt && it.ext ? `${bareTitle(it)}.${it.ext}` : bareTitle(it))
const extLine = computed(() => lay.value.showExt && !lay.value.showName)
// only cards with a note get its line; in rows of the grid and justified views, every card of a row that has one
const NOTE_H = 1.12
const noteLine = computed(() => lay.value.showNote && lay.value.view !== 'list' && S.results.notes?.size > 0)

const caption = computed(() => {
  const l = lay.value
  if (l.view === 'list') return 0
  return Math.round(rem((l.showName ? 1.54 : 0) + (l.showInfo || extLine.value ? 1.12 : 0)))
})

const isList = computed(() => lay.value.view === 'list')
const columns = computed(() => {
  const every = allColumns(S.sidebar.fields, S.contrib.columns)
  return lay.value.listCols.map((c) => ({ ...every.find((x) => x.id === c.id), w: c.w })).filter((c) => c.id)
})
const rowH = computed(() => listRowHeight(lay.value.size))
const previewCol = computed(() => columns.value.some((c) => c.render === 'thumb'))
// columns that don't fit scroll sideways rather than squeezing the name away
// in rem, as the rows' CSS has them
const NAME_MIN = 13.99
const COL_GAP = 0.84
const ROW_END = 2.38
const px = (n) => Math.round(rem(n))
const listContentW = computed(() => {
  const cols = columns.value.reduce((n, c) => n + c.w + px(COL_GAP), 0)
  return Math.max(width.value, PAD * 2 + (previewCol.value ? 0 : rowH.value * 1.25) + px(NAME_MIN) + cols + px(ROW_END))
})
// the width the columns beside the name have before the list scrolls sideways, which auto fit shares out
const columnRoom = computed(() => width.value - PAD * 2 - (previewCol.value ? 0 : rowH.value * 1.25) - px(NAME_MIN) - px(ROW_END) - columns.value.length * px(COL_GAP))
const scrollLeft = ref(0)
const contentW = computed(() => Math.max(80, (isList.value ? listContentW.value : width.value) - PAD * 2))
const gapPx = computed(() => Math.round(GRID_GAP * spacingScale()))
const headH = computed(() => Math.round(rem(2.38)))
const secGap = computed(() => Math.round(rem(1.12) * spacingScale()))
const collapsedGap = computed(() => Math.round(rem(0.28) * spacingScale()))

// ---------------------------------------------------------------- subfolders (Curio folders and Location directories)
watch(
  () => [S.scope.kind, S.scope.id],
  ([kind, id]) => {
    if (kind === 'root' || kind === 'rootdir') loadRootDirs(id)
  },
  { immediate: true },
)
const subs = computed(() => (lay.value.showSubfolders === false ? [] : scopeSubs()))
const hasSubs = computed(() => subs.value.length > 0)
const subCapH = computed(() => Math.round(rem(0.28) * spacingScale()) + Math.round(rem(1.33)) + Math.round(rem(1.12)) + Math.round(rem(0.21)))
const subLayout = computed(() => computeFolderSection(lay.value.view, subs.value.length, contentW.value, lay.value.size, gapPx.value, rowH.value, subCapH.value, headH.value, !!lay.value.subfoldersClosed, secGap.value))
const subStackPad = computed(() => (subLayout.value.faceH >= 56 ? Math.round(Math.min(8, Math.max(4, subLayout.value.faceH * 0.05))) : 0))
const subIconSize = computed(() => Math.min(48, Math.max(20, Math.round((subLayout.value.w[0] || lay.value.size) / 4))))
const subListIcon = computed(() => Math.min(28, Math.max(18, Math.round(rowH.value * 0.55))))
const itemsTop = computed(() => PAD + (hasSubs.value ? subLayout.value.height : 0))

const pickedSub = ref(null)
let firstSubClick = null
watch(() => S.scope, () => {
  pickedSub.value = null
  firstSubClick = null
})
watch(() => S.selected.length, (n) => {
  if (n) pickedSub.value = null
})
watch(() => S.status.thumbs, (n, was) => n === 0 && was > 0 && retryEmptyCovers())
function pickSub(e, sub) {
  if (e.detail === 1) firstSubClick = sub.key
  S.sideFocus = false
  pickedSub.value = sub.key
  clearSelection()
}
function goSub(sub) {
  firstSubClick = null
  pickedSub.value = null
  openSubfolder(sub)
}
function openSub(e, sub) {
  if (e.detail >= 2 && firstSubClick !== sub.key) return
  goSub(sub)
}
function toggleSubsOpen() {
  lay.value.subfoldersClosed = !lay.value.subfoldersClosed
  saveUi()
}
const subCount = (sub) => coverOf(sub)?.count ?? sub.count
function subStyle(i) {
  const F = subLayout.value
  return { left: F.x[i] + PAD + 'px', top: F.y[i] + PAD + 'px', width: F.w[i] + 'px', height: F.h[i] + 'px' }
}
function subCellText(c, sub) {
  if (c.id === 'kind') return t('grid.subfolders.kind')
  if (c.id === 'added' && sub.createdAt) return fmtDay(sub.createdAt)
  if (c.id === 'created' && sub.createdAt) return fmtDate(sub.createdAt)
  return ''
}

// ---------------------------------------------------------------- item sections (Group by + Contents below Subfolders)
const collapsedGroups = reactive(new Set())
watch(
  () => [S.scope.kind, S.scope.id, S.scope.path, S.scope.ids?.join(), lay.value.view, lay.value.group?.key, lay.value.group?.field, lay.value.group?.desc],
  () => collapsedGroups.clear(),
)
const itemGroups = computed(() => {
  void S.resultsVer
  if (S.results.groups) {
    if (S.results.groups.length) return S.results.groups
    return hasSubs.value ? [{ key: '__contents__', kind: 'contents', val: '', start: 0, count: 0 }] : null
  }
  return hasSubs.value ? [{ key: '__contents__', kind: 'contents', val: '', start: 0, count: S.results.ids.length }] : null
})
function toggleGroup(key) {
  if (collapsedGroups.has(key)) collapsedGroups.delete(key)
  else collapsedGroups.add(key)
}
function collapseAllGroups() {
  for (const s of layoutRef.value?.sections || []) collapsedGroups.add(s.key)
}
function expandAllGroups() {
  collapsedGroups.clear()
}
const secMenuTarget = ref(null)
function openSecMenu(e, sec) {
  secMenuTarget.value = sec
  secMenu.value.show(e)
}
function secMenuItems() {
  const sec = secMenuTarget.value
  if (!sec) return []
  const multi = (layoutRef.value?.sections?.length || 0) > 1
  return [
    { label: t('grid.groups.selectGroup'), icon: 'select-all', disabled: !sec.count, command: () => selectSpan(sec.start, sec.count) },
    { label: sec.collapsed ? t('grid.groups.expand') : t('grid.groups.collapse'), icon: sec.collapsed ? 'chevron-down' : 'chevron-right', command: () => toggleGroup(sec.key) },
    { separator: true, visible: multi },
    { label: t('grid.groups.collapseAll'), icon: 'chevrons-up', visible: multi, command: collapseAllGroups },
    { label: t('grid.groups.expandAll'), icon: 'chevrons-down', visible: multi && collapsedGroups.size > 0, command: expandAllGroups },
  ]
}

// computed, not set by a watcher: whatever reads it after the results change gets the layout of those results, never the one before
const layoutRef = computed(() => {
  void S.resultsVer
  void collapsedGroups.size
  const l = lay.value
  const size = l.view === 'list' ? listRowHeight(l.size) / 0.22 : l.view === 'justified' ? l.size * 0.9 : l.size
  const secOpts = itemGroups.value ? { groups: itemGroups.value, collapsed: collapsedGroups, headerH: headH.value, sectionGap: secGap.value, collapsedGap: collapsedGap.value } : null
  return layoutFor(S.results, l.view, contentW.value, size, gapPx.value, caption.value, noteLine.value ? Math.round(rem(NOTE_H)) : 0, secOpts)
})

// Every picture is a request of its own that costs about half a millisecond whatever its size, one at a time.
// A list dragged past faster than pictures arrive asks for none, or thousands would queue ahead of those for where it stops.
// fast: a screenful passes in less than this, so a picture asked for now would arrive for a cell already gone
const SCREEN_MS = 60
// speed is taken over the ticks of this long, at least SPEED_MIN_MS apart: frames come unevenly, and one alone says little
const SPEED_MS = 100
const SPEED_MIN_MS = 24
// a tick with none before it is a jump when it leaves this much of the view behind
const JUMP = 0.6
// a tick that moves less than this much of the view is the list coming to rest, however fast it was
const REST = 0.05
// cells kept ready outside the view: this much behind, and the way the list is going what it covers in LEAD_MS, between one screenful and LEAD_MAX
const READY = 400
const LEAD_MS = 250
const LEAD_MAX = 1.5
// in screenfuls, by halves, so the band doesn't change with every frame
const lead = ref(1)
const FLIGHT_GAP_MS = 100
const LAND_MS = 50
const SPARE_MS = 120
const flying = ref(false)
// pictures of the cells kept ready outside the view: held back after a leap until those in view have had their turn
const spare = ref(true)
let trail = []
let goingDown = true
let lastLeap = 0
let landTimer = 0
function moved(y, dy) {
  const now = performance.now()
  while (trail.length && now - trail[0][0] > SPEED_MS) trail.shift()
  const from = trail[0]
  trail.push([now, y])
  if (y !== scrollTop.value) goingDown = y > scrollTop.value
  const speed = from && now > from[0] ? Math.abs(y - from[1]) / (now - from[0]) : 0
  const leap = from && now - from[0] >= SPEED_MIN_MS ? dy > height.value * REST && Math.abs(y - from[1]) / (now - from[0]) > height.value / SCREEN_MS : dy > height.value * JUMP
  lead.value = leap ? 1 : Math.min(LEAD_MAX, Math.max(1, Math.round((speed * LEAD_MS * 2) / height.value) / 2))
  // one leap is a jump, shown at once; a second right after it is a drag
  flying.value = leap && now - lastLeap < FLIGHT_GAP_MS
  if (!leap) return
  lastLeap = now
  spare.value = false
  clearTimeout(landTimer)
  landTimer = setTimeout(() => {
    flying.value = false
    landTimer = setTimeout(() => (spare.value = true), SPARE_MS)
  }, LAND_MS)
}

const visibleSubs = computed(() => {
  const F = subLayout.value
  if (!F.count || F.collapsed) return []
  const y0 = scrollTop.value - PAD
  const y1 = y0 + height.value
  const idxs = visibleFolderIndices(F, y0 - READY, y1 + READY)
  const out = idxs.map((i) => ({ i, sub: subs.value[i] })).filter((x) => x.sub)
  if (!flying.value && out.length) ensureSubCovers(out.map((x) => x.sub))
  return out
})

const visibleSecHeaders = computed(() => {
  const L0 = layoutRef.value
  if (!L0?.sections?.length) return []
  const y0 = scrollTop.value - itemsTop.value
  return visibleSections(L0, y0 - READY, y0 + height.value + READY)
})

// cells whose pictures were asked for keep them while they stay on show
let pictured = new Set()
const visible = computed(() => {
  void S.itemsVer
  const L0 = layoutRef.value
  const y0 = scrollTop.value - itemsTop.value
  const y1 = y0 + height.value
  const ahead = Math.max(READY, height.value * lead.value)
  const cells = visibleItems(L0, y0 - (goingDown ? READY : ahead), y1 + (goingDown ? ahead : READY))
  const fly = flying.value
  const all = spare.value
  if (!fly) ensureItems(cells.map((c) => c.id), cells.map((c) => c.i))
  const next = new Set()
  const sectioned = !!L0.sections
  for (const c of cells) {
    c.item = S.items.get(c.id)
    c.pic = pictured.has(c.id) || (!fly && (all || (L0.y[c.i] < y1 && L0.y[c.i] + L0.h[c.i] > y0)))
    c.odd = sectioned ? ((c.i - (sectionOfIndex(L0, c.i)?.start || 0)) & 1) === 1 : (c.i & 1) === 1
    if (c.pic) next.add(c.id)
  }
  pictured = next
  return cells
})

// ---------------------------------------------------------------- list view columns
const needCells = computed(() => isList.value && columns.value.some((c) => c.cell || c.render === 'tags' || c.render === 'colors'))
const rowCells = (id) => {
  void S.cellsVer
  return S.cells.get(id)
}
watch(visible, (v) => {
  if (needCells.value) ensureCells(v.map((x) => x.id))
})
watch(needCells, (on) => {
  if (on) ensureCells(visible.value.map((x) => x.id))
})
const cellText = (c, v) => (c.value ? c.value(v.item) : c.cell ? c.cell(rowCells(v.id)) : '')
// a row's tags: as many lines as the row has room for, the ones left over counted in a last chip that names them on hover
const TAG_LINE = 1.75
const TAG_PAD = 1.26
const MORE_W = 2.8
const tagWidths = new Map()
let tagCtx = null
let tagRem = 0
function tagWidth(name) {
  if (tagRem !== remPx.value) {
    tagRem = remPx.value
    tagWidths.clear()
    tagCtx = null
  }
  let w = tagWidths.get(name)
  if (w == null) {
    if (!tagCtx) {
      tagCtx = document.createElement('canvas').getContext('2d')
      const cs = getComputedStyle(document.body)
      tagCtx.font = `${parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.85}px ${cs.fontFamily}`
    }
    w = Math.ceil(tagCtx.measureText(name).width + rem(TAG_PAD))
    tagWidths.set(name, w)
  }
  return w
}
function rowTags(id, width) {
  const tags = rowCells(id)?.tags || []
  const lines = Math.max(1, Math.floor((rowH.value - rem(0.56)) / rem(TAG_LINE)))
  const gap = rem(0.28) * spacingScale()
  const shown = []
  let line = 1
  let x = 0
  for (const t of tags) {
    const w = Math.min(tagWidth(t.name), width)
    if (x && x + w > width) {
      if (line === lines) break
      line++
      x = 0
    }
    shown.push(t)
    x += w + gap
  }
  if (shown.length === tags.length) return { shown, rest: [] }
  // room on the last line for the count
  while (shown.length && x + rem(MORE_W) > width) x -= tagWidth(shown.pop().name) + gap
  return { shown, rest: tags.slice(shown.length) }
}
const scrollbarW = computed(() => (scroller.value ? scroller.value.offsetWidth - width.value : 0))

// ---------------------------------------------------------------- Ctrl + wheel changes the thumbnail size
const ZOOM_MIN = 60
const ZOOM_MAX = 640
const ZOOM_STEP = 0.0012
function wheel(e) {
  if (!e.ctrlKey) return
  e.preventDefault()
  const l = lay.value
  l.size = Math.round(Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, l.size * (1 - e.deltaY * ZOOM_STEP))))
  saveUi()
}

const selSet = computed(() => new Set(S.selected))
const focusedId = computed(() => (S.focus != null ? S.results.ids[S.focus] : null))

let raf = 0
function onScroll() {
  if (raf) return
  raf = requestAnimationFrame(() => {
    raf = 0
    const y = scroller.value?.scrollTop || 0
    moved(y, Math.abs(y - scrollTop.value))
    scrollTop.value = y
    scrollLeft.value = scroller.value?.scrollLeft || 0
  })
}
async function ensureExpanded(i) {
  const sec = sectionOfIndex(layoutRef.value, i)
  if (sec?.collapsed) {
    collapsedGroups.delete(sec.key)
    await nextTick()
  }
}
let ro
onMounted(async () => {
  ro = new ResizeObserver(() => {
    if (scroller.value) {
      width.value = scroller.value.clientWidth
      height.value = scroller.value.clientHeight
    }
  })
  const sc = scroller.value
  if (sc) ro.observe(sc)
  // back from the viewer: the same place in the same list, or the last item viewed in the middle when it has scrolled out of sight
  const back = kept
  kept = null
  const fresh = S.scrollTo != null
  S.scrollTo = null
  if (!sc || fresh || !back || back.scope !== S.scope) return
  width.value = sc.clientWidth
  height.value = sc.clientHeight
  await nextTick()
  sc.scrollTop = back.top
  sc.scrollLeft = back.left
  const i = S.focus
  if (i == null) return
  await ensureExpanded(i)
  const L0 = layoutRef.value
  if (!L0 || L0.y[i] == null || L0.h[i] <= 0) return
  const y0 = L0.y[i] + itemsTop.value
  if (y0 < sc.scrollTop || y0 + L0.h[i] > sc.scrollTop + sc.clientHeight) sc.scrollTop = y0 + L0.h[i] / 2 - sc.clientHeight / 2
})
onBeforeUnmount(() => {
  ro?.disconnect()
  clearTimeout(landTimer)
  kept = S.viewer && scroller.value ? { scope: S.scope, top: scroller.value.scrollTop, left: scroller.value.scrollLeft } : null
})
// an item shown in its folder: in the middle, once the cards are laid out
watch(() => [S.revealAt, layoutRef.value], async ([i]) => {
  if (i == null) return
  S.revealAt = null
  await ensureExpanded(i)
  await nextTick()
  const L0 = layoutRef.value
  const sc = scroller.value
  if (sc && L0 && L0.y[i] != null && L0.h[i] > 0) sc.scrollTop = L0.y[i] + itemsTop.value + L0.h[i] / 2 - sc.clientHeight / 2
})
watch(() => S.viewer, async (v, was) => {
  if (v) hoverId.value = null
  else if (was && S.focus != null) {
    const i = S.focus
    await ensureExpanded(i)
    const L0 = layoutRef.value
    const sc = scroller.value
    if (!sc || !L0 || L0.y[i] == null || L0.h[i] <= 0) return
    const y0 = L0.y[i] + itemsTop.value
    if (y0 < sc.scrollTop || y0 + L0.h[i] > sc.scrollTop + sc.clientHeight) sc.scrollTop = y0 + L0.h[i] / 2 - sc.clientHeight / 2
  }
})
watch(
  () => S.scrollTo,
  (v) => {
    if (v == null) return
    if (scroller.value) scroller.value.scrollTop = v
    S.scrollTo = null
  },
)
// the scroller comes and goes with the empty state
watch(scroller, (a, oa) => {
  if (!ro) return
  if (oa && oa !== a) ro.unobserve(oa)
  if (a) {
    ro.observe(a)
    width.value = a.clientWidth
    height.value = a.clientHeight
  }
}, { flush: 'post' })

async function reveal(i) {
  if (i == null) return
  await ensureExpanded(i)
  const L0 = layoutRef.value
  if (!L0 || L0.y[i] == null || L0.h[i] <= 0) return
  const y0 = L0.y[i] + itemsTop.value
  const bottom = y0 + L0.h[i]
  const sc = scroller.value
  if (!sc) return
  if (y0 < sc.scrollTop) sc.scrollTop = y0 - 8
  else if (bottom > sc.scrollTop + sc.clientHeight) sc.scrollTop = bottom - sc.clientHeight + 8
}
defineExpose({
  layout: () => layoutRef.value,
  reveal,
  pageDown: (d) => {
    if (scroller.value) scroller.value.scrollTop += d * scroller.value.clientHeight * 0.9
  },
})

function cellStyle(i) {
  const L0 = layoutRef.value
  return { left: L0.x[i] + PAD + 'px', top: L0.y[i] + itemsTop.value + 'px', width: L0.w[i] + 'px', height: L0.h[i] + 'px' }
}

// a double-click must begin on the same item, so a click carried over from a replaced view (such as opening a folder) only selects
let firstClickId = null
watch(() => S.scope, () => { firstClickId = null })
function click(e, i, id) {
  if (e.detail === 1) firstClickId = id
  S.sideFocus = false
  if (e.ctrlKey || e.metaKey) toggleSelect(id, i)
  else if (e.shiftKey) selectRange(i, layoutRef.value)
  else selectOnly(id, i)
}
function dbl(e, i, id) {
  if (e.detail >= 2 && firstClickId !== id) return
  firstClickId = null
  selectOnly(id, i)
  if (S.ui.doubleClick === 'app') openItem(id, 'open')
  else openViewer(i, id)
}
function down(e, i, id) {
  if (e.button !== 0) return
  const from = S.scope.kind === 'folder' ? S.scope.id : null
  const root = S.scope.kind === 'root' ? S.scope.id : null
  const trashed = S.scope.kind === 'trash'
  beginDrag(e, { type: 'items', ids: selSet.value.has(id) ? [...S.selected] : [id], from, root, trashed }, () => `${selSet.value.has(id) ? S.selected.length : 1} item(s)`, {
    count: () => (selSet.value.has(id) ? S.selected.length : 1),
    lift: e.currentTarget.querySelector(isList.value ? '.lmain' : '.img') || undefined,
    onStart: () => {
      if (!selSet.value.has(id)) selectOnly(id, i)
      S.drag = true
    },
    onEnd: () => (S.drag = null),
    accept: (t) => t.startsWith('folder:') || t.startsWith('tag:') || t.startsWith('frame:') || (t === 'trash' && !trashed),
    describe: (target, mods) => itemDropText(target, selSet.value.has(id) ? S.selected.length : 1, S.sidebar, { shift: mods?.shift, ctrl: mods?.ctrl, from, root, trashed }),
    onLeaveWindow: () => call('start_drag', { ids: selSet.value.has(id) ? [...S.selected] : [id] }).catch((e) => toast(errorText(e), 'error')),
  })
}
function ctx(e, i, id) {
  if (!selSet.value.has(id)) selectOnly(id, i)
  menu.value.show(e)
}
// the empty space between items: what the folder or view in front of you can do
const bgMenu = ref()
function bgItems() {
  const sc = (id) => shortcutLabel(S.ui.keys, id)
  const folder = S.scope.kind === 'folder' ? S.sidebar.folders.find((f) => f.id === S.scope.id) : null
  const folderSubs = folder ? S.sidebar.folders.filter((f) => f.parent_id === folder.id).length : 0
  const anySubs = subs.value.length > 0
  const multiGroups = (layoutRef.value?.sections?.length || 0) > 1
  const l = lay.value
  if (S.scope.kind === 'trash') {
    return [
      { label: t('grid.menu.selectAll'), icon: 'select-all', shortcut: sc('item.selectAll'), visible: S.results.ids.length > 0, command: selectAll },
      { separator: true },
      { label: t('grid.menu.emptyTrash'), icon: 'trash-x', danger: true, visible: S.results.ids.length > 0, command: () => (S.dialogs.confirm = { title: t('grid.emptyTrash.title'), body: t('grid.emptyTrash.body'), okLabel: t('grid.emptyTrash.ok'), ok: () => run('item.purge', { all: true }) }) },
    ]
  }
  return [
    { label: folder ? t('grid.menu.pasteInto') : t('grid.menu.paste'), icon: 'clipboard', shortcut: sc('item.paste'), command: pasteClipboard },
    { label: t('grid.menu.selectAll'), icon: 'select-all', shortcut: sc('item.selectAll'), visible: S.results.ids.length > 0, command: selectAll },
    { separator: true },
    { label: t('grid.menu.newSubfolder'), icon: 'folder-down', visible: !!folder, command: () => startNewFolder(folder.id) },
    { label: t('grid.menu.newFolder'), icon: 'folder-plus', visible: !folder, command: () => startNewFolder() },
    { label: t('grid.menu.subfolders'), icon: 'folders', visible: anySubs, checked: !!l.subfolders, command: () => { l.subfolders = !l.subfolders; saveUi() } },
    {
      label: t('grid.menu.sortSubfolders'), icon: 'arrows-sort', visible: folderSubs > 0,
      items: [
        { label: t('grid.menu.titleAz'), icon: 'sort-ascending-letters', command: () => edit.folder('sort', { parent: folder.id, value: 'asc' }) },
        { label: t('grid.menu.titleZa'), icon: 'sort-descending-letters', command: () => edit.folder('sort', { parent: folder.id, value: 'desc' }) },
      ],
    },
    { separator: true, visible: multiGroups },
    { label: t('grid.groups.collapseAll'), icon: 'chevrons-up', visible: multiGroups, command: collapseAllGroups },
    { label: t('grid.groups.expandAll'), icon: 'chevrons-down', visible: multiGroups && collapsedGroups.size > 0, command: expandAllGroups },
  ]
}
function bgContext(e) {
  if (e.target.closest('[data-item], [data-sub], .sec-head')) return
  e.preventDefault()
  bgMenu.value.show(e)
}

// the middle button held scrolls as a browser's does, faster the further the pointer goes from where it went down; let go without moving, it is a middle click on the item there
const SCROLL_DEAD = 8
const SCROLL_SPEED = 0.01
function middleScroll(e) {
  const el = scroller.value
  const hit = e.target.closest('[data-item]')
  const id = hit ? S.results.ids.find((x) => String(x) === hit.dataset.item) : null
  const x0 = e.clientX
  const y0 = e.clientY
  let dx = 0
  let dy = 0
  let moved = false
  let frame = 0
  let last = 0
  const speed = (d, dt) => Math.sign(d) * Math.max(0, Math.abs(d) - SCROLL_DEAD) * SCROLL_SPEED * dt
  const step = (now) => {
    const dt = Math.min(now - last, 50)
    last = now
    el.scrollLeft += speed(dx, dt)
    el.scrollTop += speed(dy, dt)
    frame = requestAnimationFrame(step)
  }
  const move = (ev) => {
    dx = ev.clientX - x0
    dy = ev.clientY - y0
    if (moved || Math.abs(dx) + Math.abs(dy) < 5) return
    moved = true
    hoverId.value = null
    el.style.cursor = 'all-scroll'
    last = performance.now()
    frame = requestAnimationFrame(step)
  }
  const end = (ev) => {
    if (ev.type === 'pointerup' && ev.button !== 1) return
    el.removeEventListener('pointermove', move)
    el.removeEventListener('pointerup', end)
    el.removeEventListener('lostpointercapture', end)
    cancelAnimationFrame(frame)
    el.style.cursor = ''
    if (moved || ev.type !== 'pointerup' || id == null) return
    if (S.ui.middleClick === 'window') unreadable(id) || call('open_item_window', { ids: [id] }).catch((err) => toast(errorText(err), 'error'))
    else if (S.ui.middleClick === 'app') openItem(id, 'open')
  }
  el.setPointerCapture(e.pointerId)
  el.addEventListener('pointermove', move)
  el.addEventListener('pointerup', end)
  el.addEventListener('lostpointercapture', end)
}

// Animated GIF, WebP and PNG play while the pointer is on them (Alt holds them still), or all the
// time with Settings → Controls; the thumbnail stays the first frame so scrolling stays light.
// Videos and songs get a small player while the pointer is on them.
const hoverId = ref(null)
const isAnimated = (it) => !!it && (it.animated || it.ext === 'gif')
const playing = (it) => isAnimated(it) && it.status === 'Ok' && (S.ui.gifAlways || hoverId.value === it.id)
const hoverPlayable = (it) => !!it && S.ui.hoverPlay !== false && it.status === 'Ok' && (it.kind === 'video' || it.kind === 'audio' || !!it.motion) && !formatOff(it) && hoverId.value === it.id
function enter(e, it) {
  if (it && it.status === 'Ok' && !e.altKey && !formatOff(it) && (isAnimated(it) || it.kind === 'video' || it.kind === 'audio' || it.motion || TEXT_EXT.has(it.ext))) hoverId.value = it.id
}
// a Live Photo plays its clip; one object per clip, so the player isn't started again on every render
const clips = new Map()
function hoverItem(it) {
  if (!it.motion) return it
  let c = clips.get(it.motion)
  if (!c || c.of !== it) {
    if (clips.size > 300) clips.clear()
    clips.set(it.motion, (c = { ...it, id: it.motion, kind: 'video', of: it }))
  }
  return c
}
// files grouped into an item: what kind of group, or how many
const partBadge = (it) => (it.part_role === 'motion' ? 'LIVE' : it.part_role === 'raw' || it.part_role === 'jpeg' ? 'RAW+JPG' : `+${it.parts}`)
function leave(it) {
  if (it && hoverId.value === it.id) hoverId.value = null
}
// a picture that fails to load is asked for again a few times, then gives way to the file type's icon
const RELOAD_MS = [400, 2000, 8000]
const unloadable = reactive(new Set())
const failures = new Map()
function loadFailed(src) {
  const n = failures.get(src) || 0
  failures.set(src, n + 1)
  unloadable.add(src)
  if (n < RELOAD_MS.length) setTimeout(() => unloadable.delete(src), RELOAD_MS[n])
}
const shown = (src) => (src && !unloadable.has(src) ? src : null)
const cellSrc = (it) => shown(playing(it) ? originalUrl(it.id) : thumb(it))
// a list row's picture plays only from the middle of the Size slider up, where it is big enough to show anything
const listPlays = computed(() => lay.value.size >= (ZOOM_MIN + ZOOM_MAX) / 2)
const listSrc = (it) => (listPlays.value ? cellSrc(it) : shown(thumb(it)))
function listEnter(e, it) {
  if (listPlays.value) enter(e, it)
}
const bgClass = computed(() => 'bg-' + (lay.value.thumbBg || 'auto'))

// rubber-band selection on the empty background
function bgDown(e) {
  if (e.button === 1 && e.currentTarget === scroller.value) return middleScroll(e)
  if (e.button !== 0 || e.target !== e.currentTarget) return
  firstClickId = null
  pickedSub.value = null
  firstSubClick = null
  const rect = scroller.value.getBoundingClientRect()
  const x0 = e.clientX - rect.left + scroller.value.scrollLeft
  const y0 = e.clientY - rect.top + scroller.value.scrollTop
  const additive = e.ctrlKey || e.metaKey || e.shiftKey
  const base = additive ? [...S.selected] : []
  let moved = false
  const move = (ev) => {
    const x1 = ev.clientX - rect.left + scroller.value.scrollLeft
    const y1 = ev.clientY - rect.top + scroller.value.scrollTop
    if (!moved && Math.abs(x1 - x0) + Math.abs(y1 - y0) < 5) return
    moved = true
    const b = { x: Math.min(x0, x1), y: Math.min(y0, y1), w: Math.abs(x1 - x0), h: Math.abs(y1 - y0) }
    band.value = b
    const L0 = layoutRef.value
    const hits = visibleRange(L0, b.y - itemsTop.value, b.y + b.h - itemsTop.value).filter((i) => {
      const x = L0.x[i] + PAD
      const y = L0.y[i] + itemsTop.value
      return x < b.x + b.w && x + L0.w[i] > b.x && y < b.y + b.h && y + L0.h[i] > b.y
    })
    const set = new Set(base)
    for (const i of hits) set.add(L0.ids[i])
    S.selected = [...set]
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    band.value = null
    if (!moved && !additive) clearSelection()
    else loadDetail()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}

// a text card shows the start of the file while the pointer is on it
const TEXT_EXT = new Set(['txt', 'md', 'markdown', 'json', 'csv', 'tsv', 'log', 'xml', 'yaml', 'yml', 'ini', 'toml', 'js', 'ts', 'py', 'rs', 'css', 'sh', 'bat', 'ps1', 'c', 'cpp', 'h', 'java', 'go', 'lua', 'sql', 'srt', 'vtt'])
const snippets = new Map()
const snippetVer = ref(0)
function snippet(it) {
  if (!it || !TEXT_EXT.has(it.ext)) return null
  void snippetVer.value
  if (!snippets.has(it.id)) {
    snippets.set(it.id, '')
    fetch(`${textUrl(it.id)}/thumb`)
      .then((r) => r.text())
      .then((t) => {
        snippets.set(it.id, t.slice(0, 1500))
        if (snippets.size > 3000) snippets.delete(snippets.keys().next().value)
        snippetVer.value++
      })
      .catch(() => {})
  }
  return snippets.get(it.id)
}
const dims = (it) => (it.width && it.height && (it.kind === 'image' || it.kind === 'video') ? `${it.width} × ${it.height}` : '')
const thumb = thumbOf
</script>

<template>
  <div class="grid-wrap" :class="[bgClass, { gray: lay.grayscale, pixelated: S.ui.pixelated }]">
    <AppMenu ref="bgMenu" :model="bgItems" />
    <AppMenu ref="secMenu" :model="secMenuItems" />
    <div v-if="!S.results.ids.length && !S.searching && !hasSubs" class="empty" @contextmenu="bgContext">
      <AppIcon :name="S.scope.kind === 'trash' ? 'trash' : S.search ? 'search-off' : 'photo-off'" :size="64" class="faint" />
      <div class="muted">{{ S.scope.kind === 'trash' ? $t('grid.empty.trash') : S.search || S.queryError || filterQuery() ? $t('grid.empty.noMatches') : $t('grid.empty.nothing') }}</div>
      <div v-if="S.queryError" class="danger small">{{ S.queryError }}</div>
    </div>

    <template v-else>
      <DisplayGridListHeader v-if="isList" :pad="PAD" :scrollbar="scrollbarW" :content-width="listContentW" :room="columnRoom" :offset="scrollLeft" />
      <div ref="scroller" class="scroller" :class="{ sideways: isList }" @scroll="onScroll" @pointerdown="bgDown" @mousedown.middle.prevent @wheel="wheel" @contextmenu="bgContext">
        <div class="inner" :style="{ height: (layoutRef?.height || 0) + itemsTop + PAD + 'px', width: isList ? listContentW + 'px' : undefined }" @pointerdown="bgDown">
          <template v-if="hasSubs">
            <DisplayGridSectionHeader
              :title="$t('grid.subfolders.title')"
              :count="subs.length"
              :collapsed="!!lay.subfoldersClosed"
              :style="{ left: PAD + 'px', top: PAD + subLayout.headerY + 'px', width: contentW + 'px' }"
              @toggle="toggleSubsOpen"
            >
              <template #right>
                <label class="show-all" @click.stop><Checkbox v-model="lay.subfolders" binary @change="saveUi" /> {{ $t('grid.subfolders.include') }}</label>
              </template>
            </DisplayGridSectionHeader>
            <template v-if="isList">
              <div
                v-for="v in visibleSubs"
                :key="v.sub.key"
                :data-sub="v.sub.key"
                :data-drop="v.sub.drop || undefined"
                class="lrow sub-lrow"
                :class="{ sel: pickedSub === v.sub.key, odd: v.i % 2 }"
                :style="subStyle(v.i)"
                tabindex="0"
                @click="pickSub($event, v.sub)"
                @dblclick="openSub($event, v.sub)"
                @keydown.enter.stop.prevent="goSub(v.sub)"
              >
                <div class="lmain" :class="{ bare: previewCol }">
                  <div v-if="!previewCol" class="lpic" :style="{ width: rowH * 1.25 + 'px' }">
                    <img v-if="coverPic(v.sub)" :src="coverPic(v.sub)" class="sub-lpic" draggable="false" />
                    <AppIcon v-else :name="v.sub.icon || 'folder'" :size="subListIcon" class="faint" :style="{ color: v.sub.color || undefined }" />
                  </div>
                  <span class="lname sub-lname">
                    <span class="ellipsis">{{ v.sub.name }}</span>
                    <span class="sub-lcount">{{ $t('grid.subfolders.items', { count: subCount(v.sub).toLocaleString() }, subCount(v.sub)) }}</span>
                  </span>
                </div>
                <div v-for="c in columns" :key="c.id" class="lcell" :class="{ mono: c.mono }" :style="{ width: c.w + 'px' }" :data-col="c.id">
                  <div v-if="c.render === 'thumb'" class="lpic" :style="{ height: rowH - 10 + 'px', margin: 0 }">
                    <img v-if="coverPic(v.sub)" :src="coverPic(v.sub)" class="sub-lpic" draggable="false" />
                    <AppIcon v-else :name="v.sub.icon || 'folder'" :size="subListIcon" class="faint" :style="{ color: v.sub.color || undefined }" />
                  </div>
                  <span v-else class="ltext">{{ subCellText(c, v.sub) }}</span>
                </div>
              </div>
            </template>
            <template v-else>
              <div
                v-for="v in visibleSubs"
                :key="v.sub.key"
                :data-sub="v.sub.key"
                :data-drop="v.sub.drop || undefined"
                class="sub-card"
                :class="{ sel: pickedSub === v.sub.key }"
                :style="subStyle(v.i)"
                tabindex="0"
                @click="pickSub($event, v.sub)"
                @dblclick="openSub($event, v.sub)"
                @keydown.enter.stop.prevent="goSub(v.sub)"
              >
                <div class="sub-stack" :style="{ height: subLayout.faceH + 'px', paddingTop: subStackPad + 'px' }">
                  <template v-if="subStackPad">
                    <span class="sub-edge far" />
                    <span class="sub-edge near" />
                  </template>
                  <div class="sub-face">
                    <img v-if="coverPic(v.sub)" :src="coverPic(v.sub)" draggable="false" />
                    <AppIcon v-else :name="v.sub.icon || 'folder'" :size="subIconSize" class="faint" :style="{ color: v.sub.color || undefined }" />
                  </div>
                </div>
                <div class="sub-name ellipsis">{{ v.sub.name }}</div>
                <div class="sub-count ellipsis">{{ $t('grid.subfolders.items', { count: subCount(v.sub).toLocaleString() }, subCount(v.sub)) }}</div>
              </div>
            </template>
          </template>

          <DisplayGridSectionHeader
            v-for="sec in visibleSecHeaders"
            :key="sec.key"
            :title="sectionTitle(sec)"
            :count="sec.count"
            :collapsed="sec.collapsed"
            :style="{ left: PAD + 'px', top: sec.headerY + itemsTop + 'px', width: contentW + 'px' }"
            @toggle="toggleGroup(sec.key)"
            @contextmenu="openSecMenu($event, sec)"
          />

          <template v-if="isList">
            <div v-for="v in visible" :key="v.id" :data-item="v.id" class="lrow" :class="{ sel: selSet.has(v.id), odd: v.odd }" :style="cellStyle(v.i)"
                 @click="click($event, v.i, v.id)" @dblclick="dbl($event, v.i, v.id)" @pointerdown="down($event, v.i, v.id)" @contextmenu.prevent="ctx($event, v.i, v.id)">
              <template v-if="v.item">
                <div class="lmain" :class="{ bare: previewCol }">
                  <div v-if="!previewCol" class="lpic" :class="{ explicit: v.item.explicit }" :style="{ width: rowH * 1.25 + 'px' }" @mouseenter="listEnter($event, v.item)" @mouseleave="leave(v.item)">
                    <img v-if="v.pic && listSrc(v.item)" :src="listSrc(v.item)" draggable="false" @error="loadFailed(listSrc(v.item))" />
                    <AppIcon v-else-if="!listSrc(v.item)" :name="KIND_ICON[v.item.kind] || 'file'" :size="24" class="faint" />
                    <HoverPlayer v-if="hoverPlayable(v.item)" :key="v.id" :item="hoverItem(v.item)" compact />
                  </div>
                  <span class="lname">{{ shownTitle(v.item) }}</span>
                  <AppIcon v-if="v.item.status !== 'Ok'" name="alert-triangle" class="warn" />
                </div>
                <div v-for="c in columns" :key="c.id" class="lcell" :class="{ mono: c.mono }" :style="{ width: c.w + 'px' }" :data-col="c.id">
                  <div v-if="c.render === 'thumb'" class="lpic" :class="{ explicit: v.item.explicit }" :style="{ height: rowH - 10 + 'px', margin: 0 }" @mouseenter="listEnter($event, v.item)" @mouseleave="leave(v.item)">
                    <img v-if="v.pic && listSrc(v.item)" :src="listSrc(v.item)" draggable="false" @error="loadFailed(listSrc(v.item))" />
                    <AppIcon v-else-if="!listSrc(v.item)" :name="KIND_ICON[v.item.kind] || 'file'" :size="24" class="faint" />
                    <HoverPlayer v-if="hoverPlayable(v.item)" :key="v.id" :item="hoverItem(v.item)" compact />
                  </div>
                  <div v-else-if="c.render === 'tags'" class="ltags"><template v-for="r in [rowTags(v.id, c.w)]" :key="v.id"><span v-for="t in r.shown" :key="t.id" class="ltag" :style="tagTint(t.name)">{{ t.name }}</span><span v-if="r.rest.length" class="ltag more" v-tooltip.bottom="r.rest.map((t) => t.name).join(', ')">+{{ r.rest.length }}</span></template></div>
                  <span v-else-if="c.render === 'rating'" class="lstars"><AppIcon v-for="n in v.item.rating" :key="n" name="star-filled" :size="14" /></span>
                  <FontSample v-else-if="c.render === 'font'" :item="v.item" :height="rowH - 10" />
                  <span v-else-if="c.render === 'colors'" class="ldots"><i v-for="(p, k) in (rowCells(v.id)?.asset?.palette || []).slice(0, 6)" :key="k" :style="{ background: `rgb(${p.color.join(',')})` }" /></span>
                  <span v-else class="ltext">{{ cellText(c, v) }}</span>
                </div>
              </template>
            </div>
          </template>
          <template v-else>
            <div v-for="v in visible" :key="v.id" :data-item="v.id" class="cell" :class="{ sel: selSet.has(v.id), focus: focusedId === v.id && selSet.size > 1 }" :style="cellStyle(v.i)"
                 @click="click($event, v.i, v.id)" @dblclick="dbl($event, v.i, v.id)" @pointerdown="down($event, v.i, v.id)" @contextmenu.prevent="ctx($event, v.i, v.id)" @mouseenter="enter($event, v.item)" @mouseleave="leave(v.item)">
              <div class="img" :class="{ explicit: v.item?.explicit }" :style="{ height: `calc(100% - ${layoutRef.below?.[v.i] ?? caption}px)` }">
                <template v-if="v.item">
                  <img v-if="v.pic && cellSrc(v.item)" :src="cellSrc(v.item)" draggable="false" loading="eager" @error="loadFailed(cellSrc(v.item))" />
                  <pre v-else-if="hoverId === v.id && snippet(v.item)" class="snip">{{ snippet(v.item) }}</pre>
                  <div v-else-if="!cellSrc(v.item)" class="ph"><AppIcon :name="KIND_ICON[v.item.kind] || 'file'" :size="Math.min(48, Math.max(20, lay.size / 5))" /><span>{{ v.item.ext.toUpperCase() }}</span></div>
                  <HoverPlayer v-if="hoverPlayable(v.item)" :key="v.id" :item="hoverItem(v.item)" />
                  <span v-if="v.item.parts && !hoverPlayable(v.item)" class="badge pbadge" v-tooltip="$t('grid.badges.parts', { count: v.item.parts })">{{ partBadge(v.item) }}</span>
                  <span v-if="badgeShown(lay) && !playing(v.item) && !hoverPlayable(v.item)" class="badge">{{ v.item.ext.toUpperCase() }}</span>
                  <span v-if="v.item.status !== 'Ok'" class="status" :class="v.item.status === 'Offline' ? 'off' : 'miss'" v-tooltip="v.item.status === 'Offline' ? $t('grid.badges.offline') : $t('grid.badges.missing')"><AppIcon name="alert-triangle" :size="13" /></span>
                </template>
              </div>
              <div v-if="(caption || noteLine) && v.item" class="cap">
                <div v-if="lay.showName" class="cap-name ellipsis">{{ shownTitle(v.item) }}</div>
                <div v-if="lay.showInfo || extLine" class="cap-info ellipsis">{{ [extLine ? v.item.ext.toUpperCase() : '', lay.showInfo ? dims(v.item) : ''].filter(Boolean).join(' \u00b7 ') }}</div>
                <div v-if="noteLine && S.results.notes.has(v.id)" class="cap-info cap-note ellipsis">{{ v.item.note }}</div>
              </div>
            </div>
          </template>
          <div v-if="band" class="band" :style="{ left: band.x + 'px', top: band.y + 'px', width: band.w + 'px', height: band.h + 'px' }" />
        </div>
      </div>
    </template>
    <ItemMenu ref="menu" />
  </div>
</template>

<style scoped>
.grid-wrap { position: absolute; inset: 0; display: flex; flex-direction: column; }
/* always reserves its scrollbar: one that came and went could narrow the layout enough to lose it again, shaking forever */
.scroller { flex: 1; overflow-y: scroll; overflow-x: hidden; position: relative; }
.scroller.sideways { overflow-x: auto; }
.inner { position: relative; width: 100%; }
.show-all { display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); font-size: 0.92rem; color: var(--text-muted); cursor: pointer; }
.sub-card { position: absolute; text-align: center; cursor: pointer; border-radius: calc(8px * var(--r)); outline: none; contain: layout style; }
.sub-card.sel .sub-face, .sub-card.drop-hover .sub-face { outline: 2px solid var(--selection-border); outline-offset: 2px; }
.sub-card:hover:not(.sel) .sub-face { outline: 1px solid var(--input-border); outline-offset: 2px; }
.sub-card:focus-visible .sub-face { outline: 2px solid var(--accent); outline-offset: 2px; }
.sub-card.sel .sub-name { color: var(--accent); }
.sub-stack { position: relative; width: 100%; }
.sub-edge { position: absolute; height: round(0.7rem, 1px); border-radius: calc(6px * var(--r)) calc(6px * var(--r)) 0 0; background: var(--surface-2); }
.sub-edge.far { left: 10%; right: 10%; top: 0; opacity: 0.45; }
.sub-edge.near { left: 5%; right: 5%; top: 0.18rem; opacity: 0.75; }
.sub-face { position: relative; width: 100%; height: 100%; border-radius: calc(6px * var(--r)); overflow: hidden; background: var(--surface-2); display: flex; align-items: center; justify-content: center; }
.sub-face img { width: 100%; height: 100%; object-fit: cover; display: block; }
.sub-name { margin-top: calc(0.28rem * var(--g)); line-height: round(1.33rem, 1px); font-size: 0.92rem; color: var(--grid-caption); }
.sub-count { line-height: round(1.12rem, 1px); font-size: 0.85rem; color: var(--text-faint); }
.sub-lrow { cursor: pointer; outline: none; }
.sub-lrow:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.lname.sub-lname { display: flex; align-items: baseline; gap: calc(0.56rem * var(--g)); -webkit-line-clamp: unset; }
.sub-lcount { flex: none; font-size: 0.85rem; color: var(--text-faint); }
.cell { position: absolute; border-radius: calc(6px * var(--r)); contain: layout style; }
.cell .img { position: relative; width: 100%; border-radius: calc(5px * var(--r)); overflow: hidden; background: var(--grid-item); display: flex; align-items: center; justify-content: center; }
.cell img { width: 100%; height: 100%; object-fit: cover; display: block; }
.img.explicit > img, .img.explicit :deep(video), .lpic.explicit > img, .lpic.explicit :deep(video) { filter: blur(10px); transform: scale(1.06); }
.gray img { filter: grayscale(1); }
.gray .img.explicit > img, .gray .lpic.explicit > img { filter: grayscale(1) blur(10px); }
.pixelated img { image-rendering: pixelated; }
.bg-dark .cell .img { background: #111214; }
.bg-light .cell .img { background: #e8e9eb; }
.cell.sel .img { outline: 2px solid var(--selection-border); outline-offset: 2px; }
.cell.focus .img { outline-width: 3px; }
.cell:hover:not(.sel) .img { outline: 1px solid var(--input-border); outline-offset: 2px; }
.snip { position: absolute; inset: 0; margin: 0; padding: calc(1.82rem * var(--g)) calc(0.7rem * var(--g)) calc(0.56rem * var(--g)); overflow: hidden; font-family: var(--mono-font); font-size: 0.68rem; line-height: 1.45; color: var(--text-muted); white-space: pre-wrap; word-break: break-word; background: var(--surface); text-align: left; }
.ph { display: flex; flex-direction: column; align-items: center; gap: calc(0.42rem * var(--g)); color: var(--text-faint); font-size: 0.85rem; }
.badge { position: absolute; left: 0.42rem; top: 0.42rem; font-size: 0.72rem; font-weight: 600; padding: 1px calc(0.35rem * var(--g)); border-radius: calc(4px * var(--r)); background: rgba(0,0,0,.55); color: #fff; border: 1px solid rgba(255,255,255,.25); }
.pbadge { top: auto; bottom: 0.42rem; }
.status { position: absolute; right: 0.42rem; top: 0.42rem; width: round(1.4rem, 1px); height: round(1.4rem, 1px); border-radius: var(--round); display: flex; align-items: center; justify-content: center; color: #111; }
.status.off { background: var(--warning); }
.status.miss { background: var(--danger); }
.cap { text-align: center; padding-top: calc(0.28rem * var(--g)); }
.cap-name { font-size: 0.92rem; color: var(--grid-caption); }
.cell.sel .cap-name { color: var(--accent); }
.cap-info { font-size: 0.85rem; color: var(--text-faint); }
.band { position: absolute; border: 1px solid var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); pointer-events: none; }
.lrow { position: absolute; display: flex; align-items: center; gap: calc(0.84rem * var(--g)); padding-right: 2.38rem; border-radius: calc(6px * var(--r)); font-size: 1rem; }
.lrow.odd { background: color-mix(in srgb, var(--hover) 45%, transparent); }
.lrow:hover { background: var(--hover); }
.lrow.sel { background: var(--selection); }
.lmain.bare { padding-left: calc(0.7rem * var(--g)); }
.lmain { flex: 1; min-width: 0; height: 100%; display: flex; align-items: center; gap: calc(0.84rem * var(--g)); }
/* the thumbnail keeps its own shape, inside a fixed slot so names line up */
.lpic { position: relative; height: calc(100% - 10px); flex: none; margin-left: calc(0.35rem * var(--g)); display: flex; align-items: center; justify-content: center; }
.lpic img { max-width: 100%; max-height: 100%; object-fit: contain; border-radius: calc(3px * var(--r)); display: block; }
.lpic img.sub-lpic { width: 100%; height: 100%; object-fit: cover; border-radius: calc(4px * var(--r)); }
.lname { flex: 1; min-width: 0; line-height: 1.3; overflow: hidden; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; word-break: break-word; }
.lcell { flex: none; min-width: 0; font-size: 0.92rem; color: var(--text-muted); overflow: hidden; }
.lcell.mono { font-size: 0.85rem; }
.ltext { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ltags { display: flex; flex-wrap: wrap; gap: calc(0.28rem * var(--g)); overflow: hidden; }
.ltag { max-width: 100%; overflow: hidden; text-overflow: ellipsis; padding: 0 calc(0.49rem * var(--g)); line-height: round(1.47rem, 1px); border-radius: calc(5px * var(--r)); border: 0; background: color-mix(in srgb, var(--text) 9%, transparent); color: var(--text); font-size: 0.85rem; white-space: nowrap; }
.ltag[style*='--tint'] { background: color-mix(in srgb, var(--tint) 18%, transparent); }
.ltag.more { flex: none; color: var(--text-muted); cursor: default; }
.lstars { display: inline-flex; gap: 1px; color: var(--star); }
.ldots { display: inline-flex; gap: calc(0.21rem * var(--g)); }
.ldots i { width: round(0.91rem, 1px); height: round(0.91rem, 1px); border-radius: var(--round); display: block; }
.warn { color: var(--warning); }
.empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: calc(0.84rem * var(--g)); }
.small { font-size: 0.85rem; max-width: 60%; text-align: center; }
</style>
