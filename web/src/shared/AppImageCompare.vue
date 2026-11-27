<script>
import { computed, ref } from 'vue'

// as in Curio's viewer: this much wheel or this many pixels of a right-button drag upwards doubles the size
const WHEEL_DOUBLING = 500
const GESTURE_DOUBLING_PX = 160
const ZOOM_MIN = 0.05
const ZOOM_MAX = 40

/** Wheel or right-drag zoom about the pointer, and drag to pan once zoomed in, over the element the handlers are on. */
function useZoom() {
  const zoom = ref(1)
  const x = ref(0)
  const y = ref(0)
  const reset = () => ((zoom.value = 1), (x.value = 0), (y.value = 0))
  /** To `next`, keeping the point `px`, `py` (from the middle) where it is; at fit size or below the picture sits in the middle. */
  const zoomAt = (next, px = 0, py = 0) => {
    next = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next))
    const k = next / zoom.value
    x.value = next <= 1 ? 0 : px - (px - x.value) * k
    y.value = next <= 1 ? 0 : py - (py - y.value) * k
    zoom.value = next
  }
  const fromMiddle = (e, el) => {
    const r = el.getBoundingClientRect()
    return [e.clientX - r.left - r.width / 2, e.clientY - r.top - r.height / 2]
  }
  const wheel = (e) => {
    e.preventDefault()
    zoomAt(zoom.value * 2 ** (-e.deltaY / WHEEL_DOUBLING), ...fromMiddle(e, e.currentTarget))
  }
  let drag = null
  const down = (e) => {
    const el = e.currentTarget
    if (e.button === 2) {
      const [px, py] = fromMiddle(e, el)
      const z0 = zoom.value
      const x0 = x.value
      const y0 = y.value
      const sy = e.clientY
      drag = (ev) => {
        zoom.value = z0
        x.value = x0
        y.value = y0
        zoomAt(z0 * 2 ** ((sy - ev.clientY) / GESTURE_DOUBLING_PX), px, py)
      }
    } else if (e.button === 0 && zoom.value > 1) {
      const sx = e.clientX - x.value
      const sy = e.clientY - y.value
      drag = (ev) => ((x.value = ev.clientX - sx), (y.value = ev.clientY - sy))
    } else return
    el.setPointerCapture(e.pointerId)
  }
  const up = () => (drag = null)
  const style = computed(() => ({ transform: `translate(${x.value}px, ${y.value}px) scale(${zoom.value})` }))
  return { zoom, style, reset, zoomAt, on: { onWheel: wheel, onPointerdown: down, onPointermove: (e) => drag?.(e), onPointerup: up, onPointercancel: up, onDblclick: reset, onContextmenu: (e) => e.preventDefault() } }
}

// the zoom slider is logarithmic, as the viewer's is: every part of it multiplies the size by the same amount
const PERCENT_MIN = 10
const PERCENT_MAX = 1000
const toPercent = (v) => PERCENT_MIN * Math.exp((v / 100) * Math.log(PERCENT_MAX / PERCENT_MIN))
const toSlider = (pc) => (Math.log(Math.max(PERCENT_MIN, pc) / PERCENT_MIN) / Math.log(PERCENT_MAX / PERCENT_MIN)) * 100
</script>

<script setup>
import { onBeforeUnmount, onMounted, watch } from 'vue'
import Slider from 'openvue/slider'
import AppIcon from './AppIcon.vue'
import { t } from './i18n.js'

/** Two or more pictures, by address, compared: a slider, side by side, one over the other with its opacity, or their difference; wheel or right-drag zooms, drag pans.
 * Side by side, the pictures zoom and pan together unless unlinked. `toolbar` adds the zoom slider, actual size, fit and, side by side, the link; `labels` names the pictures on the stage. */
const props = defineProps({ images: { type: Array, default: () => [] }, mode: { type: String, default: 'slider' }, toolbar: { type: Boolean, default: false }, labels: { type: Array, default: () => [] }, linked: { type: Boolean, default: true } })

const box = ref(null)
const split = ref(50)
const opacity = ref(50)
// one per picture shown side by side, made as more are shown
const zooms = [useZoom()]
const linkedNow = ref(props.linked)
watch(() => props.linked, (v) => (linkedNow.value = v))
// side by side and unlinked, each picture has a zoom of its own; otherwise the first one's moves them all
const zoomOf = (i) => {
  if (props.mode !== 'side' || linkedNow.value) return zooms[0]
  while (zooms.length <= i) zooms.push(useZoom())
  return zooms[i]
}
const resetAll = () => zooms.forEach((z) => z.reset())
// the same pictures given again, as a page redrawn after a rating, keep their zoom
watch(() => props.images, (now, was) => {
  if (!was || now.length !== was.length || now.some((u, i) => u !== was[i])) resetAll()
})
// the share of the first picture's own pixels shown at fit size, so the zoom reads as a percentage of them
const fit = ref(1)
const measure = () => {
  const img = box.value?.querySelector('img')
  if (img?.naturalWidth && img.offsetWidth) fit.value = (img.offsetWidth * (window.devicePixelRatio || 1)) / img.naturalWidth
}
let watcher = null
onMounted(() => {
  watcher = new ResizeObserver(measure)
  if (box.value) watcher.observe(box.value)
})
onBeforeUnmount(() => watcher?.disconnect())
const percent = computed(() => zooms[0].zoom.value * fit.value * 100)
const zoomAll = (z) => zooms.forEach((s) => s.zoomAt(z))
function link() {
  linkedNow.value = !linkedNow.value
  if (linkedNow.value) zooms.slice(1).forEach((z) => z.reset())
}
let dragging = false
function slide(e) {
  if (!dragging) return
  const r = box.value.getBoundingClientRect()
  split.value = Math.min(100, Math.max(0, ((e.clientX - r.left) / r.width) * 100))
}
function slideKey(e) {
  const k = e.shiftKey ? 10 : 2
  const to = { ArrowLeft: split.value - k, ArrowRight: split.value + k, Home: 0, End: 100 }[e.key]
  if (to == null) return
  e.preventDefault()
  split.value = Math.min(100, Math.max(0, to))
}
function grab(e) {
  dragging = true
  e.stopPropagation()
  e.currentTarget.setPointerCapture(e.pointerId)
}
const a = computed(() => props.images[0])
const b = computed(() => props.images[1])
</script>

<template>
  <div class="curio-compare" :class="mode">
    <div ref="box" class="curio-compare-stage" v-bind="mode === 'side' ? {} : zooms[0].on">
      <span v-if="!a" class="faint">{{ t('shared.compare.empty') }}</span>
      <div v-else-if="mode === 'side'" class="curio-compare-side">
        <div v-for="(u, i) in images" :key="i" class="curio-compare-cell" v-bind="zoomOf(i).on">
          <div class="curio-compare-zoom" :style="zoomOf(i).style.value"><img :src="u" draggable="false" alt="" @load="measure" /></div>
          <span v-if="labels[i]" class="curio-compare-label left">{{ labels[i] }}</span>
        </div>
      </div>
      <template v-else>
        <!-- the slider cuts in the stage's own space, outside the zoom, so the cut stays on the handle and each side shows one picture only -->
        <div v-if="b && mode === 'slider'" class="curio-compare-clip" :style="{ clipPath: `inset(0 ${100 - split}% 0 0)` }"><div class="curio-compare-zoom" :style="zooms[0].style.value"><img :src="a" draggable="false" alt="" @load="measure" /></div></div>
        <div v-else class="curio-compare-zoom" :style="zooms[0].style.value"><img :src="a" draggable="false" alt="" @load="measure" /></div>
        <div v-if="b && mode === 'slider'" class="curio-compare-clip" :style="{ clipPath: `inset(0 0 0 ${split}%)` }"><div class="curio-compare-zoom top" :style="zooms[0].style.value"><img :src="b" draggable="false" alt="" @load="measure" /></div></div>
        <div v-if="b && mode === 'overlay'" class="curio-compare-zoom top" :style="{ ...zooms[0].style.value, opacity: opacity / 100 }"><img :src="b" draggable="false" alt="" @load="measure" /></div>
        <div v-if="b && mode === 'difference'" class="curio-compare-zoom top" :style="{ ...zooms[0].style.value, mixBlendMode: 'difference' }"><img :src="b" draggable="false" alt="" @load="measure" /></div>
        <div v-if="b && mode === 'slider'" class="curio-compare-handle" :style="{ left: `${split}%` }" tabindex="0" role="slider" :aria-label="t('shared.compare.divider')" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="Math.round(split)" @keydown="slideKey" @pointerdown="grab" @pointermove="slide" @pointerup="dragging = false"><span><AppIcon name="arrows-horizontal" :size="16" /></span></div>
        <span v-if="labels[0]" class="curio-compare-label left">{{ labels[0] }}</span>
        <span v-if="labels[1]" class="curio-compare-label right">{{ labels[1] }}</span>
      </template>
    </div>
    <Slider v-if="mode === 'overlay'" v-model="opacity" :min="0" :max="100" class="curio-compare-opacity" :aria-label="t('shared.compare.opacity')" />
    <div v-if="toolbar && a" class="curio-compare-tools">
      <Slider :model-value="toSlider(percent)" :min="0" :max="100" :step="0.25" class="curio-compare-range" :aria-label="t('shared.compare.zoom')" @update:model-value="(v) => zoomAll(toPercent(v) / 100 / fit)" />
      <span class="faint mono curio-compare-pct">{{ Math.round(percent) }}%</span>
      <span class="grow" />
      <button v-if="mode === 'side'" class="icon-btn" :class="{ on: linkedNow }" :title="linkedNow ? t('shared.compare.unlink') : t('shared.compare.link')" :aria-pressed="String(linkedNow)" @click="link"><AppIcon :name="linkedNow ? 'link' : 'unlink'" :size="18" /></button>
      <button class="icon-btn" :title="t('shared.compare.actualSize')" @click="zoomAll(1 / fit)"><AppIcon name="circle-dashed-number-1" :size="18" /></button>
      <button class="icon-btn" :title="t('shared.compare.fit')" @click="resetAll"><AppIcon name="arrows-maximize" :size="18" /></button>
    </div>
  </div>
</template>

<style>
.curio-compare-handle:focus-visible { outline: none; }
.curio-compare-handle:focus-visible span { box-shadow: 0 0 0 2px var(--accent); }
.curio-compare { display: flex; flex-direction: column; gap: 0.56rem; height: 100%; min-height: round(13.99rem, 1px); }
.curio-compare-stage { position: relative; flex: 1; min-height: 0; overflow: hidden; background: var(--viewer-bg); border-radius: calc(8px * var(--r)); touch-action: none; }
.curio-compare-clip { position: absolute; inset: 0; }
.curio-compare-zoom { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; transform-origin: center; }
.curio-compare-zoom img { max-width: 100%; max-height: 100%; object-fit: contain; user-select: none; }
.curio-compare-side { position: absolute; inset: 0; display: flex; gap: 0.28rem; }
.curio-compare-cell { position: relative; flex: 1; overflow: hidden; }
.curio-compare-handle { position: absolute; top: 0; bottom: 0; width: 0.14rem; margin-left: -1px; background: #fff; box-shadow: 0 0 4px rgba(0,0,0,.5); cursor: ew-resize; touch-action: none; }
.curio-compare-handle span { position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); width: round(2.1rem, 1px); height: round(2.1rem, 1px); border-radius: var(--round); background: #fff; color: #222; display: flex; align-items: center; justify-content: center; box-shadow: 0 1px 4px rgba(0,0,0,.4); }
.curio-compare-opacity { width: round(13.99rem, 1px); align-self: center; margin: calc(0.42rem * var(--g)) 0; }
.curio-compare-tools { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); flex: none; }
.curio-compare-range { width: round(12.59rem, 1px); margin: 0 calc(0.56rem * var(--g)); }
.curio-compare-pct { min-width: round(3.22rem, 1px); font-size: 0.92rem; }
.curio-compare-label { position: absolute; top: 0.7rem; max-width: 45%; padding: 0.14rem 0.56rem; border-radius: calc(5px * var(--r)); background: rgba(0, 0, 0, .55); color: #fff; font-size: 0.85rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; pointer-events: none; }
.curio-compare-label.left { left: 0.7rem; }
.curio-compare-label.right { right: 0.7rem; }
</style>
