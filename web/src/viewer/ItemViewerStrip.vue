<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { ensureItems } from '../library/results.js'
import { saveUi } from '../app/preferences.js'
import { stepViewer, viewerPosition, viewerResultAt, viewerResults } from './session.js'
import { GRID_GAP, layoutFor, visibleItems } from '../shared/gridLayout.js'
import AppIcon from '../shared/AppIcon.vue'
import AppMenu from '../shared/AppMenu.vue'
import { KIND_ICON, thumbOf } from '../formats/index.js'
import { t } from '../shared/i18n.js'

// The neighbours of an open item, in a row below it or a column beside it.
const props = defineProps({ side: { type: String, default: 'bottom' } })
const STRIP_SIZE = 84
const PAD = 8
const across = computed(() => props.side === 'bottom')
const strip = ref()
const menu = ref()
const at = ref(0)
const length = ref(800)
const activePos = computed(() => {
  void S.resultsVer
  return viewerPosition().pos
})
const layout = computed(() => {
  void S.resultsVer
  return layoutFor(viewerResults(), across.value ? 'filmstrip' : 'column', 0, STRIP_SIZE, GRID_GAP, 0)
})

const visible = computed(() => {
  void S.itemsVer
  const cells = visibleItems(layout.value, at.value - 300, at.value + length.value + 300)
  ensureItems(cells.map((c) => c.id), cells.map((c) => viewerResultAt(c.i)))
  for (const c of cells) c.item = S.items.get(c.id)
  return cells
})
const thumb = thumbOf
function center() {
  const L0 = layout.value
  const i = activePos.value
  const el = strip.value
  if (!L0 || i < 0 || !el) return
  if (across.value) el.scrollLeft = Math.max(0, L0.x[i] + L0.w[i] / 2 - el.clientWidth / 2)
  else el.scrollTop = Math.max(0, L0.y[i] + L0.h[i] / 2 - el.clientHeight / 2)
}
watch([activePos, across], center, { flush: 'post' })
const measure = () => (length.value = (across.value ? strip.value?.clientWidth : strip.value?.clientHeight) || 800)
let ro
onMounted(() => {
  ro = new ResizeObserver(measure)
  ro.observe(strip.value)
  center()
})
onBeforeUnmount(() => ro?.disconnect())
const scrolled = () => (at.value = across.value ? strip.value.scrollLeft : strip.value.scrollTop)
const go = (i) => stepViewer(i - activePos.value)
function wheel(e) {
  if (across.value) strip.value.scrollLeft += e.deltaY || e.deltaX
  else strip.value.scrollTop += e.deltaY
}
const SIDES = [['left', 'layout-sidebar'], ['bottom', 'layout-bottombar'], ['right', 'layout-sidebar-right']]
const sideItems = () => SIDES.map(([v, icon]) => ({ label: t(`viewer.strip.${v}`), icon, current: props.side === v, command: () => { S.ui.viewerStripSide = v; saveUi() } }))
function cellStyle(i) {
  const L0 = layout.value
  return across.value ? { left: L0.x[i] + PAD + 'px', width: L0.w[i] + 'px' } : { top: L0.y[i] + PAD + 'px', height: L0.h[i] + 'px' }
}
</script>

<template>
  <div ref="strip" class="viewer-strip" data-no-scrollbar :class="[across ? 'across' : 'down', 'side-' + side]" @scroll="scrolled" @pointerdown.stop @dblclick.stop @wheel.stop.prevent="wheel" @contextmenu.prevent.stop="menu.show($event)">
    <div class="inner" :style="across ? { width: (layout?.width || 0) + PAD * 2 + 'px' } : { height: (layout?.height || 0) + PAD * 2 + 'px' }">
      <div v-for="v in visible" :key="v.id" class="thumb" :class="{ on: activePos === v.i }" :style="cellStyle(v.i)" @click="go(v.i)">
        <img v-if="thumb(v.item)" :src="thumb(v.item)" draggable="false" />
        <AppIcon v-else :name="KIND_ICON[v.item?.kind] || 'file'" :size="24" class="faint" />
      </div>
    </div>
    <AppMenu ref="menu" :model="sideItems" />
  </div>
</template>

<style scoped>
.viewer-strip { flex: none; background: var(--panel); scrollbar-width: none; }
.viewer-strip.across { height: 100px; overflow-x: auto; overflow-y: hidden; border-top: 1px solid var(--border); }
.viewer-strip.down { width: 100px; overflow-x: hidden; overflow-y: auto; }
.viewer-strip.side-right { border-left: 1px solid var(--border); }
.viewer-strip.side-left { border-right: 1px solid var(--border); }
.viewer-strip::-webkit-scrollbar { display: none; }
.inner { position: relative; }
.across .inner { height: 100%; }
.down .inner { width: 100%; }
.thumb { position: absolute; border-radius: calc(5px * var(--r)); overflow: hidden; background: var(--grid-item); display: flex; align-items: center; justify-content: center; cursor: pointer; }
.across .thumb { top: 8px; height: 84px; }
.down .thumb { left: 8px; width: 84px; }
.thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
.thumb.on { outline: 2px solid var(--selection-border); outline-offset: 1px; }
</style>
