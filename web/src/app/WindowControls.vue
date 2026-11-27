<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { MAC_BUTTONS, getCurrentWindow } from '../platform/windowFrame.js'
import { S } from './state.js'
import { toggleOnTop } from './windowChrome.js'
import AppIcon from '../shared/AppIcon.vue'

// `main`: a library's window, whose buttons the person may hide; other windows keep theirs, having nothing else to close them
// `right`: how far from the window's right edge they sit, past the panes there when they end the toolbar
const props = defineProps({ edgesOnly: Boolean, main: Boolean, right: { type: Number, default: 0 } })

// The window draws its own frame: pin, minimise, maximise, close; on a Mac the last three are MacWindowButtons, which the window's layout places, and the window's corners rounded.
const win = getCurrentWindow()
const fullScreen = ref(false)
const bar = ref()
let stop = null
// the toolbar under the controls keeps clear of exactly their width
const sized = new ResizeObserver(([e]) => document.documentElement.style.setProperty('--wc-width', Math.ceil(e.borderBoxSize?.[0]?.inlineSize ?? e.target.offsetWidth) + 'px'))
async function refresh() {
  S.maximized = await win.isMaximized().catch(() => false)
  if (MAC_BUTTONS) fullScreen.value = await win.isFullscreen().catch(() => false)
}
const shown = computed(() => !S.fullscreen && !props.edgesOnly && (S.ui.windowButtons !== false || !props.main))
// a Mac's close, minimise and zoom (MacWindowButtons): every window has them, Settings too, and a library's window while its buttons are shown
watch(() => MAC_BUTTONS && !S.fullscreen && !fullScreen.value && (props.edgesOnly || shown.value), (on) => (S.lights = on), { immediate: true })
watch(() => MAC_BUTTONS && !fullScreen.value, (on) => document.documentElement.classList.toggle('mac-window', on), { immediate: true })
// hidden in Settings: the toolbar takes the whole width
watch(bar, (el, old) => {
  if (old) sized.unobserve(old)
  if (el) sized.observe(el)
  else document.documentElement.style.removeProperty('--wc-width')
})
let gone = false
onMounted(async () => {
  await refresh()
  const unlisten = await win.onResized(refresh).catch(() => null)
  // gone while the listener was being set up
  if (gone) unlisten?.()
  else stop = unlisten
})
onBeforeUnmount(() => {
  gone = true
  stop?.()
  sized.disconnect()
  document.documentElement.style.removeProperty('--wc-width')
})
const EDGES = ['North', 'South', 'East', 'West', 'NorthEast', 'NorthWest', 'SouthEast', 'SouthWest']
const resize = (edge) => win.startResizeDragging(edge).catch(() => {})
</script>

<template>
  <div v-if="shown" ref="bar" class="win-controls" :style="right ? { right: right + 'px' } : null">
    <button class="wc" :class="{ on: S.onTop }" v-tooltip.bottom="$t('app.windowControls.alwaysOnTop')" @click="toggleOnTop"><AppIcon :name="S.onTop ? 'pinned-filled' : 'pin'" :size="16" /></button>
    <template v-if="!MAC_BUTTONS">
      <span class="wc-sep" />
      <button class="wc" :aria-label="$t('app.windowControls.minimize')" @click="win.minimize()"><AppIcon name="minus" :size="16" /></button>
      <button class="wc" :aria-label="$t('app.windowControls.maximize')" @click="win.toggleMaximize()"><AppIcon :name="S.maximized ? 'squares' : 'square'" :size="14" /></button>
      <button class="wc close" :aria-label="$t('app.windowControls.close')" @click="win.close()"><AppIcon name="x" :size="16" /></button>
    </template>
  </div>
  <!-- a Mac's window is resized at its edges by the system -->
  <template v-if="!MAC_BUTTONS && !S.maximized && !S.fullscreen">
    <div v-for="e in EDGES" :key="e" class="win-edge" :class="e" @pointerdown.prevent="resize(e)" />
  </template>
</template>

<style scoped>
.win-controls { position: fixed; top: 0; right: 0; z-index: 900; display: flex; align-items: center; gap: 0.14rem; height: round(3.36rem, 1px); padding: 0 calc(0.56rem * var(--g)); }
.wc { display: flex; align-items: center; justify-content: center; width: round(2.38rem, 1px); height: round(2.1rem, 1px); border: 0; border-radius: calc(6px * var(--r)); background: transparent; color: var(--text-muted); cursor: pointer; }
.wc:hover { background: var(--hover); color: var(--text); }
.wc.on { color: var(--accent); }
.wc.close:hover { background: var(--danger); color: #fff; }
.wc-sep { width: 1px; height: round(1.12rem, 1px); margin: 0 calc(0.28rem * var(--g)); background: var(--border); }
.win-edge { position: fixed; z-index: 950; }
.North { top: 0; left: 0.56rem; right: 0.56rem; height: 0.28rem; cursor: ns-resize; }
.South { bottom: 0; left: 0.56rem; right: 0.56rem; height: 0.28rem; cursor: ns-resize; }
.East { right: 0; top: 0.56rem; bottom: 0.56rem; width: 0.28rem; cursor: ew-resize; }
.West { left: 0; top: 0.56rem; bottom: 0.56rem; width: 0.28rem; cursor: ew-resize; }
.NorthEast { top: 0; right: 0; width: round(0.56rem, 1px); height: round(0.56rem, 1px); cursor: nesw-resize; }
.SouthWest { bottom: 0; left: 0; width: round(0.56rem, 1px); height: round(0.56rem, 1px); cursor: nesw-resize; }
.NorthWest { top: 0; left: 0; width: round(0.56rem, 1px); height: round(0.56rem, 1px); cursor: nwse-resize; }
.SouthEast { bottom: 0; right: 0; width: round(0.56rem, 1px); height: round(0.56rem, 1px); cursor: nwse-resize; }
</style>
