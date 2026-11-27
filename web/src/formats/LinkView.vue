<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { call, on } from '../platform/ipc.js'
import { thumbOf } from './index.js'
import { closeViewer } from '../viewer/viewerControls.js'
import AppIcon from '../shared/AppIcon.vue'

// Saved links: YouTube plays in place, other pages load in the viewer, in a web view of their own laid over the stage.
const props = defineProps({ item: Object, compact: Boolean })
const link = ref(null)
const thumb = computed(() => thumbOf(props.item))
const ytId = computed(() => {
  const m = (link.value || '').match(/(?:youtube\.com\/(?:watch\?(?:.*&)?v=|shorts\/|embed\/|live\/|v\/)|youtu\.be\/|youtube-nocookie\.com\/embed\/)([A-Za-z0-9_-]{11})/)
  return m ? m[1] : null
})
watch(() => props.item?.id, async (id) => {
  link.value = null
  try {
    const url = (await call('asset_detail', { id })).asset.source_url
    if (props.item?.id === id) link.value = url
  } catch {
    if (props.item?.id === id) link.value = null
  }
}, { immediate: true })
const openInApp = () => call('open_web', { url: link.value, title: props.item?.title })
const openOutside = () => call('open_url', { url: link.value })

const page = computed(() => !props.compact && !!link.value && !ytId.value && /^https?:/i.test(link.value))
const slot = ref(null)
// the address the viewer should show, the one its web view has, and where that lies
let want = null
let shown = null
let last = ''

// the page's web view lies above everything this window draws, so it steps aside while a menu or dialog is over its place
const OVERLAYS = '.p-dialog-mask, .p-popover, .p-contextmenu, .p-tieredmenu, .p-menu-overlay, .p-select-overlay, .p-autocomplete-overlay, .p-datepicker-panel, .p-colorpicker-panel'
const meets = (a, b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
/** Where the page goes, in the window's physical pixels; null while it is covered or has no room. */
function rect() {
  const el = slot.value
  if (!el) return null
  const r = el.getBoundingClientRect()
  if (r.width < 2 || r.height < 2) return null
  for (const o of document.querySelectorAll(OVERLAYS)) if (meets(o.getBoundingClientRect(), r)) return null
  const k = window.devicePixelRatio || 1
  return [Math.round(r.left * k), Math.round(r.top * k), Math.round(r.width * k), Math.round(r.height * k)]
}
function sync() {
  if (!want) {
    if (shown) call('viewer_page', { op: 'close' }).catch(() => {})
    shown = null
    last = ''
    return
  }
  const r = rect()
  const key = JSON.stringify(r)
  if (shown !== want) {
    // a page opens once it has a place to show in
    if (!r) return
    shown = want
    last = key
    call('viewer_page', { op: 'open', url: want, rect: r }).catch(() => (shown = null))
  } else if (key !== last) {
    last = key
    call('viewer_page', { op: 'place', rect: r }).catch(() => {})
  }
}
watch([page, link], ([show, url]) => {
  want = show ? url : null
  sync()
}, { flush: 'post', immediate: true })

// the stage moves with panels, the strip and the window without changing size, so its place is checked often
const PLACE_EVERY_MS = 150
let timer = null
let ro = null
let mo = null
let unlisten = null
let gone = false
onMounted(async () => {
  timer = setInterval(sync, PLACE_EVERY_MS)
  ro = new ResizeObserver(sync)
  if (slot.value) ro.observe(slot.value)
  mo = new MutationObserver(sync)
  mo.observe(document.body, { childList: true })
  const off = await on('viewer-page-key', (key) => key === 'Escape' && shown && closeViewer())
  if (gone) off()
  else unlisten = off
})
onBeforeUnmount(() => {
  gone = true
  clearInterval(timer)
  ro?.disconnect()
  mo?.disconnect()
  unlisten?.()
  want = null
  sync()
})
</script>

<template>
  <iframe v-if="ytId && !compact" class="frame fills-stage" :src="`https://www.youtube-nocookie.com/embed/${ytId}?autoplay=1&vq=hq1080`" referrerpolicy="strict-origin-when-cross-origin" allow="autoplay; encrypted-media; picture-in-picture; fullscreen" allowfullscreen />
  <div v-else-if="page" ref="slot" class="page-slot fills-stage">
    <img v-if="thumb" :src="thumb" class="pic" draggable="false" />
  </div>
  <div v-else class="link-area">
    <img v-if="thumb" :src="thumb" class="pic" :class="{ compact }" draggable="false" />
    <AppIcon v-else :name="ytId ? 'brand-youtube' : 'world'" :size="compact ? 40 : 72" class="faint" />
    <span v-if="ytId" class="play" :class="{ compact }"><AppIcon name="player-play-filled" :size="compact ? 18 : 28" /></span>
    <div v-if="!compact && link" class="link-bar">
      <span class="ellipsis mono">{{ link }}</span>
      <button class="lb" @click="openInApp"><AppIcon name="app-window" /> {{ $t('formats.link.inApp') }}</button>
      <button class="lb" @click="openOutside"><AppIcon name="external-link" /> {{ $t('formats.link.browser') }}</button>
    </div>
  </div>
</template>

<style scoped>
.frame { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: #000; }
.link-area { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
/* the thumbnail shows until the page has drawn over it, and while a menu covers its place */
.page-slot { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; background: var(--bg); }
.pic { max-width: 100%; max-height: 100%; object-fit: contain; }
.pic.compact { border-radius: calc(6px * var(--r)); }
.play { position: absolute; display: flex; align-items: center; justify-content: center; width: round(3.36rem, 1px); height: round(3.36rem, 1px); border-radius: var(--round); background: rgba(0,0,0,.55); color: #fff; pointer-events: none; }
.play.compact { width: round(2.38rem, 1px); height: round(2.38rem, 1px); }
.link-bar { position: absolute; left: 50%; bottom: 1.12rem; transform: translateX(-50%); display: flex; align-items: center; gap: calc(0.56rem * var(--g)); max-width: 90%; padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)) calc(0.42rem * var(--g)) calc(0.84rem * var(--g)); border-radius: calc(10px * var(--r)); background: var(--surface-2); border: 1px solid var(--border); }
.link-bar .mono { font-size: 0.85rem; color: var(--text-muted); max-width: 29.37rem; user-select: text; }
.lb { display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); border: 0; background: var(--button); color: var(--text); font: inherit; cursor: pointer; white-space: nowrap; }
.lb:hover { background: var(--button-hover); }
</style>
