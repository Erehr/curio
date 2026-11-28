<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { L } from '../layout/arrangement.js'
import { S } from '../app/state.js'
import { ensureItems, viewRequest } from '../library/results.js'
import { selectOnly, toggleSelect, selectRange, clearSelection, loadDetail } from '../library/selection.js'
import { openViewer } from '../viewer/session.js'
import { openItem } from '../library/lifecycle.js'
import { unreadable } from '../library/navigation.js'
import { saveUi } from '../app/preferences.js'
import { formatOff, thumbOf } from '../formats/index.js'
import { originalUrl } from '../platform/fileUrls.js'
import { call, errorText } from '../platform/ipc.js'
import { toast } from '../app/notify.js'
import { beginDrag } from '../app/dragDrop.js'
import { itemDropText } from '../app/drops.js'
import { loadRootDirs } from '../sidebar/sidebar.js'
import ItemMenu from '../library/ItemMenu.vue'
import AppIcon from '../shared/AppIcon.vue'
import HoverPlayer from '../formats/HoverPlayer.vue'
import { foldersIn, placeSpectrum, pointAt, pointsIn, readSpectrum, stepSpectrum } from './spectrumLayout.js'
import { spectrumView as V, showSpectrum, spectrumCenter, resetSpectrum } from './spectrumView.js'
import { scopeSubs, openSubfolder, coverOf, coverPic, coverColor, ensureSubCovers, retryEmptyCovers, subCoversVer } from './groupLabels.js'

// The Spectrum view: pictures on a disc by hue and lightness (spectrumLayout.js). The middle button moves about, and the wheel zooms about the pointer.
// Moving about moves one layer under the pictures; which pictures are there is worked out again only every STEP units, so a pan redraws nothing in between.
const lay = computed(() => L())
const host = ref()
const menu = ref()
const band = ref(null)
const width = ref(800)
const height = ref(600)
const loaded = ref(false)
// what each result is in the layout, -1 for none
let pointOf = new Int32Array(0)
let indexOf = new Int32Array(0)
const ZOOM_MIN = 60
const ZOOM_MAX = 640
const ZOOM_STEP = 0.0012
const STEP = 2

const unit = computed(() => lay.value.size)
watch([width, height, unit], () => {
  V.w = width.value / unit.value
  V.h = height.value / unit.value
}, { immediate: true })

// ---------------------------------------------------------------- subfolders at the center of the disc
watch(
  () => [S.scope.kind, S.scope.id],
  ([kind, id]) => {
    if (kind === 'root' || kind === 'rootdir') loadRootDirs(id)
  },
  { immediate: true },
)
const subs = computed(() => (lay.value.showSubfolders === false ? [] : scopeSubs()))
const hasSubs = computed(() => subs.value.length > 0)
watch(subs, (list) => { if (list.length) ensureSubCovers(list) }, { immediate: true })
function hexSpectrum(hex) {
  if (!hex || typeof hex !== 'string' || !/^#[0-9a-f]{6}$/i.test(hex)) return null
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const d = max - min
  const h = d === 0 ? 0 : (((max === r ? (g - b) / d : max === g ? 2 + (b - r) / d : 4 + (r - g) / d) * 60) + 360) % 360
  return [h, d * 100, ((max + min) / 2) * 100]
}
const subColor = (sub) => coverColor(sub) || hexSpectrum(sub.color)
const pickedSub = ref(null)
let firstSubClick = null
watch(() => S.scope, () => {
  pickedSub.value = null
  firstSubClick = null
  hoverId.value = null
})
watch(() => S.selected.length, (n) => {
  if (n) pickedSub.value = null
})
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
const subCount = (sub) => coverOf(sub)?.count ?? sub.count

// ---------------------------------------------------------------- the layout of what the view finds
let seq = 0
let points = null
function place() {
  if (!points) return
  const aspects = S.results.aspects
  const list = subs.value
  const A = placeSpectrum(points, (j) => aspects[points.index[j]], { dark: !!lay.value.spectrumDark, folders: list.length, folderColors: list.map(subColor) })
  showSpectrum(A, points.rgb, lastKey, { points, pointOf, indexOf, resultsVer: S.resultsVer, results: S.results })
}
let lastKey = ''
async function load() {
  const mine = ++seq
  const req = viewRequest()
  try {
    const buf = await call('spectrum_colors', { req })
    if (mine !== seq) return
    const ids = S.results.ids
    const at = new Map()
    for (let i = 0; i < ids.length; i++) at.set(ids[i], i)
    const p = readSpectrum(buf, (id) => at.get(id))
    pointOf = new Int32Array(ids.length).fill(-1)
    p.index.forEach((i, j) => (pointOf[i] = j))
    indexOf = p.index
    V.missing = ids.length - p.n
    points = p
    lastKey = JSON.stringify(req)
    place()
    loaded.value = true
  } catch (e) {
    if (mine === seq) toast(errorText(e), 'error')
  }
}
watch(() => S.resultsVer, load)
watch(() => lay.value.spectrumDark, place)
watch(() => [subs.value.map((s) => `${s.key}:${s.color || ''}`).join(','), subCoversVer.value], place)
// the middle of the screen stays over the disc, so it can't be lost off the edge
watch(() => [V.x, V.y, V.layout], () => {
  const A = V.layout
  if (!A?.n && !A?.folders?.count) return
  const [cx, cy] = spectrumCenter(A)
  const R = Math.max(2, Math.max(A.max[0] - cx, cx - A.min[0], A.max[1] - cy, cy - A.min[1]) * 0.9)
  const dx = V.x - cx
  const dy = V.y - cy
  const d = Math.hypot(dx, dy)
  if (d <= R) return
  V.x = cx + (dx * R) / d
  V.y = cy + (dy * R) / d
})
// palettes come with thumbnails: those made since the last load are placed once the queue has run out
watch(() => S.status.thumbs, (n, was) => {
  if (n === 0 && was > 0) {
    retryEmptyCovers()
    load()
  }
})

// ---------------------------------------------------------------- what is on screen
const toUnits = (px, py) => [V.x + (px - width.value / 2) / unit.value, V.y + (py - height.value / 2) / unit.value]
// the screen's place by steps: what is shown changes only when it moves into another
const win = computed(() => `${Math.floor(V.x / STEP)},${Math.floor(V.y / STEP)}`)
function around(margin) {
  const [qx, qy] = win.value.split(',').map(Number)
  return [qx * STEP - V.w / 2 - margin, qy * STEP - V.h / 2 - margin, (qx + 1) * STEP + V.w / 2 + margin, (qy + 1) * STEP + V.h / 2 + margin]
}
const visible = computed(() => {
  void S.itemsVer
  const A = V.layout
  if (!A) return []
  const out = pointsIn(A, ...around(1)).map((j) => ({ j, i: indexOf[j], id: S.results.ids[indexOf[j]], item: null }))
  for (const v of out) v.item = S.items.get(v.id)
  return out
})
const visibleSubs = computed(() => {
  const A = V.layout
  if (!A?.folders?.count) return []
  const list = foldersIn(A, ...around(1)).map((i) => ({ i, sub: subs.value[i] })).filter((x) => x.sub)
  if (list.length) ensureSubCovers(list.map((x) => x.sub))
  return list
})
// rows are read half a screen ahead every way, so a picture's address is known before it comes into view
watch(() => [win.value, V.layout, V.w, V.h], () => {
  const A = V.layout
  if (!A) return
  const js = pointsIn(A, ...around(Math.max(V.w, V.h) / 2))
  ensureItems(js.map((j) => S.results.ids[indexOf[j]]), js.map((j) => indexOf[j]))
}, { immediate: true })
const plane = computed(() => ({ transform: `translate3d(${Math.round(width.value / 2 - V.x * unit.value)}px, ${Math.round(height.value / 2 - V.y * unit.value)}px, 0)` }))
function cellStyle(v) {
  const A = V.layout
  const u = unit.value
  const k = v.j * 3
  return {
    left: A.x[v.j] * u + 'px',
    top: (A.row[v.j] - 0.5 + A.gap / 2) * u + 'px',
    width: A.w[v.j] * u + 'px',
    height: (1 - A.gap) * u + 'px',
    background: `rgb(${V.rgb[k]},${V.rgb[k + 1]},${V.rgb[k + 2]})`,
  }
}
const subTileH = computed(() => Math.round((1 - (V.layout?.gap ?? 0.06)) * unit.value))
const subStackPad = computed(() => (subTileH.value >= 56 ? Math.round(Math.min(8, Math.max(4, subTileH.value * 0.05))) : 0))
const subIconSize = computed(() => Math.min(48, Math.max(18, Math.round(subTileH.value / 3))))
const subCompact = computed(() => subTileH.value < 88)
function subStyle(i) {
  const A = V.layout
  const F = A.folders
  const u = unit.value
  return {
    left: F.x[i] * u + 'px',
    top: (F.row[i] - 0.5 + A.gap / 2) * u + 'px',
    width: F.w * u + 'px',
    height: (1 - A.gap) * u + 'px',
  }
}
const selSet = computed(() => new Set(S.selected))
const focusedId = computed(() => (S.focus != null ? S.results.ids[S.focus] : null))

// ---------------------------------------------------------------- hover playback (video, audio, Live Photos and animated pictures)
const hoverId = ref(null)
const isAnimated = (it) => !!it && (it.animated || it.ext === 'gif')
const playing = (it) => isAnimated(it) && it.status === 'Ok' && (S.ui.gifAlways || hoverId.value === it.id)
const hoverPlayable = (it) => !!it && S.ui.hoverPlay !== false && it.status === 'Ok' && (it.kind === 'video' || it.kind === 'audio' || !!it.motion) && !formatOff(it) && hoverId.value === it.id
function enter(e, it) {
  if (it && it.status === 'Ok' && !e.altKey && !formatOff(it) && (isAnimated(it) || it.kind === 'video' || it.kind === 'audio' || it.motion)) hoverId.value = it.id
}
function leave(it) {
  if (it && hoverId.value === it.id) hoverId.value = null
}
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
const cellSrc = (it) => (it ? (playing(it) ? originalUrl(it.id) : thumbOf(it)) : null)

// ---------------------------------------------------------------- pointer: as in the grid, with the middle button moving about
function pointIndex(e) {
  const r = host.value.getBoundingClientRect()
  const [x, y] = toUnits(e.clientX - r.left, e.clientY - r.top)
  const j = V.layout ? pointAt(V.layout, x, y) : null
  return j == null ? null : indexOf[j]
}
let firstClickId = null
watch(() => S.scope, () => { firstClickId = null })
function click(e, i, id) {
  if (e.detail === 1) firstClickId = id
  S.sideFocus = false
  if (e.ctrlKey || e.metaKey) toggleSelect(id, i)
  else if (e.shiftKey) selectRange(i)
  else selectOnly(id, i)
}
function dbl(e, i, id) {
  if (e.detail >= 2 && firstClickId !== id) return
  firstClickId = null
  selectOnly(id, i)
  if (S.ui.doubleClick === 'app') openItem(id, 'open')
  else openViewer(i, id)
}
function dragItems(e, i, id) {
  if (e.button !== 0) return
  const from = S.scope.kind === 'folder' ? S.scope.id : null
  const root = S.scope.kind === 'root' ? S.scope.id : null
  const trashed = S.scope.kind === 'trash'
  const ids = () => (selSet.value.has(id) ? [...S.selected] : [id])
  beginDrag(e, { type: 'items', ids: ids(), from, root, trashed }, () => `${ids().length} item(s)`, {
    count: () => ids().length,
    onStart: () => {
      if (!selSet.value.has(id)) selectOnly(id, i)
      hoverId.value = null
      S.drag = true
    },
    onEnd: () => (S.drag = null),
    accept: (t) => t.startsWith('folder:') || t.startsWith('tag:') || t.startsWith('frame:') || (t === 'trash' && !trashed),
    describe: (target, mods) => itemDropText(target, ids().length, S.sidebar, { shift: mods?.shift, ctrl: mods?.ctrl, from, root, trashed }),
    onLeaveWindow: () => call('start_drag', { ids: ids() }).catch((err) => toast(errorText(err), 'error')),
  })
}
function context(e, i, id) {
  if (!selSet.value.has(id)) selectOnly(id, i)
  menu.value.show(e)
}

function down(e) {
  if (e.button === 1) return pan(e)
  if (e.button === 0 && !e.target.closest('[data-item], [data-sub]')) {
    firstClickId = null
    pickedSub.value = null
    firstSubClick = null
    rubberBand(e)
  }
}
// the middle button held moves the disc; let go where it went down, it is a middle click on what is there
function pan(e) {
  e.preventDefault()
  const x0 = e.clientX
  const y0 = e.clientY
  const from = [V.x, V.y]
  let moved = false
  const move = (ev) => {
    if (!moved && Math.abs(ev.clientX - x0) + Math.abs(ev.clientY - y0) < 5) return
    if (!moved) hoverId.value = null
    moved = true
    V.x = from[0] - (ev.clientX - x0) / unit.value
    V.y = from[1] - (ev.clientY - y0) / unit.value
  }
  const up = (ev) => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    if (moved) return
    const i = pointIndex(ev)
    if (i == null) return
    const id = S.results.ids[i]
    if (S.ui.middleClick === 'window') unreadable(id) || call('open_item_window', { ids: [id] }).catch((err) => toast(errorText(err), 'error'))
    else if (S.ui.middleClick === 'app') openItem(id, 'open')
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
function rubberBand(e) {
  const r = host.value.getBoundingClientRect()
  const x0 = e.clientX - r.left
  const y0 = e.clientY - r.top
  const additive = e.ctrlKey || e.metaKey || e.shiftKey
  const base = additive ? [...S.selected] : []
  let moved = false
  const move = (ev) => {
    const x1 = ev.clientX - r.left
    const y1 = ev.clientY - r.top
    if (!moved && Math.abs(x1 - x0) + Math.abs(y1 - y0) < 5) return
    moved = true
    band.value = { x: Math.min(x0, x1), y: Math.min(y0, y1), w: Math.abs(x1 - x0), h: Math.abs(y1 - y0) }
    const [ax, ay] = toUnits(band.value.x, band.value.y)
    const [bx, by] = toUnits(band.value.x + band.value.w, band.value.y + band.value.h)
    const set = new Set(base)
    for (const j of pointsIn(V.layout, ax, ay, bx, by)) set.add(S.results.ids[indexOf[j]])
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
// the wheel zooms about the pointer, along with the thumbnail size
function wheel(e) {
  e.preventDefault()
  if (!e.deltaY) return
  const r = host.value.getBoundingClientRect()
  const [px, py] = toUnits(e.clientX - r.left, e.clientY - r.top)
  const before = unit.value
  const l = lay.value
  const next = Math.round(Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, l.size * (1 - e.deltaY * ZOOM_STEP))))
  if (next === l.size) return
  l.size = next
  const k = before / l.size
  V.x = px - (px - V.x) * k
  V.y = py - (py - V.y) * k
  saveUi()
}

// ---------------------------------------------------------------- what the keyboard asks of a view
function reveal(i, center = false) {
  const j = pointOf[i]
  const A = V.layout
  if (j == null || j < 0 || !A) return
  const x0 = A.x[j]
  const x1 = x0 + A.w[j]
  const y0 = A.row[j] - 0.5 + A.gap / 2
  const y1 = A.row[j] + 0.5 - A.gap / 2
  const pad = 8 / unit.value
  const left = V.x - V.w / 2
  const right = V.x + V.w / 2
  const top = V.y - V.h / 2
  const bottom = V.y + V.h / 2
  if (center) {
    if (x0 - pad < left || x1 + pad > right || y0 - pad < top || y1 + pad > bottom) {
      V.x = (x0 + x1) / 2
      V.y = A.row[j]
    }
    return
  }
  if (x1 - x0 + pad * 2 >= V.w) V.x = (x0 + x1) / 2
  else if (x0 - pad < left) V.x = x0 - pad + V.w / 2
  else if (x1 + pad > right) V.x = x1 + pad - V.w / 2
  if (y1 - y0 + pad * 2 >= V.h) V.y = A.row[j]
  else if (y0 - pad < top) V.y = y0 - pad + V.h / 2
  else if (y1 + pad > bottom) V.y = y1 + pad - V.h / 2
}
function step(from, dx, dy) {
  const A = V.layout
  if (!A?.n) return null
  // nothing chosen: the picture nearest the middle of the screen
  if (from == null || pointOf[from] < 0) {
    let best = 0
    let bestD = Infinity
    for (let j = 0; j < A.n; j++) {
      const d = (A.x[j] + A.w[j] / 2 - V.x) ** 2 + (A.row[j] - V.y) ** 2
      if (d < bestD) ((bestD = d), (best = j))
    }
    return indexOf[best]
  }
  const j = stepSpectrum(A, pointOf[from], dx, dy)
  return j == null ? null : indexOf[j]
}
defineExpose({
  layout: () => ({ mode: 'spectrum', n: S.results.ids.length, step }),
  reveal,
  pageDown: (d) => (V.y += d * V.h * 0.9),
})
// what the grid is asked for: Home and End go to the middle, an item shown in its folder is brought into view
watch(() => S.scrollTo, (v) => {
  if (v == null) return
  S.scrollTo = null
  const [cx, cy] = spectrumCenter(V.layout)
  V.x = cx
  V.y = cy
}, { immediate: true })
watch(() => [S.revealAt, V.layout], ([i, A]) => {
  if (i == null || !A) return
  S.revealAt = null
  reveal(i, true)
}, { immediate: true })

watch(() => S.viewer, (v, was) => {
  if (v) hoverId.value = null
  else if (was && S.focus != null) reveal(S.focus, true)
})

let ro
onMounted(() => {
  ro = new ResizeObserver(() => {
    width.value = host.value.clientWidth
    height.value = host.value.clientHeight
  })
  ro.observe(host.value)
  width.value = host.value.clientWidth
  height.value = host.value.clientHeight
  if (V.layout && V.points && V.resultsVer === S.resultsVer) {
    points = V.points
    pointOf = V.pointOf
    indexOf = V.indexOf
    lastKey = V.key
    loaded.value = true
  } else load()
})
onBeforeUnmount(() => {
  ro?.disconnect()
  if (!S.viewer) resetSpectrum()
})
</script>

<template>
  <div ref="host" class="spectrum" :class="{ gray: lay.grayscale, pixelated: S.ui.pixelated }" @pointerdown="down" @wheel="wheel">
    <div class="plane" :style="plane">
      <div
        v-for="v in visibleSubs"
        :key="v.sub.key"
        :data-sub="v.sub.key"
        :data-drop="v.sub.drop || undefined"
        class="sub-cell"
        :class="{ sel: pickedSub === v.sub.key, compact: subCompact }"
        :style="subStyle(v.i)"
        tabindex="0"
        v-tooltip.bottom="`${v.sub.name} · ${$t('grid.subfolders.items', { count: subCount(v.sub).toLocaleString() }, subCount(v.sub))}`"
        @click="pickSub($event, v.sub)"
        @dblclick="openSub($event, v.sub)"
        @keydown.enter.stop.prevent="goSub(v.sub)"
      >
        <div class="sub-stack" :style="{ paddingTop: subStackPad + 'px' }">
          <template v-if="subStackPad">
            <span class="sub-edge far" />
            <span class="sub-edge near" />
          </template>
          <div class="sub-face">
            <img v-if="coverPic(v.sub)" :src="coverPic(v.sub)" draggable="false" decoding="async" />
            <AppIcon v-else :name="v.sub.icon || 'folder'" :size="subIconSize" class="faint" :style="{ color: v.sub.color || undefined }" />
            <div class="sub-bar">
              <AppIcon :name="v.sub.icon || 'folder'" :size="13" class="sub-bar-ico" :style="{ color: v.sub.color || undefined }" />
              <span class="sub-bar-name ellipsis">{{ v.sub.name }}</span>
              <span v-if="!subCompact" class="sub-bar-count">{{ subCount(v.sub).toLocaleString() }}</span>
            </div>
          </div>
        </div>
      </div>

      <div v-for="v in visible" :key="v.id" :data-item="v.id" class="cell" :class="{ sel: selSet.has(v.id), focus: focusedId === v.id && selSet.size > 1, explicit: v.item?.explicit }" :style="cellStyle(v)"
           @click="click($event, v.i, v.id)" @dblclick="dbl($event, v.i, v.id)" @pointerdown="dragItems($event, v.i, v.id)" @contextmenu.prevent="context($event, v.i, v.id)" @mouseenter="enter($event, v.item)" @mouseleave="leave(v.item)">
        <img v-if="v.item && cellSrc(v.item)" :src="cellSrc(v.item)" draggable="false" decoding="async" />
        <HoverPlayer v-if="hoverPlayable(v.item)" :key="v.id" :item="hoverItem(v.item)" />
      </div>
    </div>
    <div v-if="band" class="band" :style="{ left: band.x + 'px', top: band.y + 'px', width: band.w + 'px', height: band.h + 'px' }" />
    <div v-if="loaded && !S.searchingAnew && !V.layout?.n && !hasSubs" class="empty muted">{{ $t('grid.spectrum.empty') }}</div>
    <ItemMenu ref="menu" />
  </div>
</template>

<style scoped>
.spectrum { position: absolute; inset: 0; overflow: hidden; touch-action: none; user-select: none; }
.plane { position: absolute; left: 0; top: 0; will-change: transform; }
.sub-cell { position: absolute; cursor: pointer; outline: none; contain: layout style; }
.sub-stack { position: relative; width: 100%; height: 100%; }
.sub-edge { position: absolute; height: round(0.7rem, 1px); border-radius: calc(5px * var(--r)) calc(5px * var(--r)) 0 0; background: var(--surface-2); }
.sub-edge.far { left: 10%; right: 10%; top: 0; opacity: 0.45; }
.sub-edge.near { left: 5%; right: 5%; top: 0.18rem; opacity: 0.75; }
.sub-face { position: relative; width: 100%; height: 100%; border-radius: calc(5px * var(--r)); overflow: hidden; background: var(--surface-2); display: flex; align-items: center; justify-content: center; }
.sub-face img { width: 100%; height: 100%; object-fit: cover; display: block; }
.sub-cell.sel .sub-face, .sub-cell.drop-hover .sub-face { outline: 2px solid var(--selection-border); outline-offset: 2px; }
.sub-cell:hover:not(.sel) .sub-face { outline: 1px solid var(--input-border); outline-offset: 2px; }
.sub-cell:focus-visible .sub-face { outline: 2px solid var(--accent); outline-offset: 2px; }
.sub-bar { position: absolute; left: 0; right: 0; bottom: 0; display: flex; align-items: center; gap: calc(0.28rem * var(--g)); padding: calc(0.35rem * var(--g)) calc(0.49rem * var(--g)); background: linear-gradient(transparent, rgba(0, 0, 0, 0.78) 45%); color: #fff; font-size: 0.85rem; }
.sub-cell.compact .sub-bar { padding: calc(0.21rem * var(--g)) calc(0.35rem * var(--g)); }
.sub-bar-ico { flex: none; opacity: 0.9; }
.sub-bar-name { flex: 1; min-width: 0; font-weight: 500; }
.sub-bar-count { flex: none; font-size: 0.78rem; opacity: 0.8; }
.cell { position: absolute; border-radius: calc(4px * var(--r)); overflow: hidden; contain: layout style; }
.cell img { width: 100%; height: 100%; object-fit: cover; display: block; }
.cell.explicit { overflow: hidden; }
.cell.explicit > img, .cell.explicit :deep(video) { filter: blur(10px); transform: scale(1.06); }
.cell :deep(.hover-player) { border-radius: inherit; }
.cell.sel { outline: 2px solid var(--selection-border); outline-offset: 2px; overflow: visible; }
.cell.explicit.sel { overflow: hidden; }
.cell.sel img { border-radius: inherit; }
.cell.focus { outline-width: 3px; }
.cell:hover:not(.sel) { outline: 1px solid var(--input-border); outline-offset: 2px; }
.gray img { filter: grayscale(1); }
.pixelated img { image-rendering: pixelated; }
.band { position: absolute; border: 1px solid var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); pointer-events: none; }
.empty { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
</style>
