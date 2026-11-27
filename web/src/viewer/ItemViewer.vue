<script setup>
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { openItem } from '../library/lifecycle.js'
import { ensureItems } from '../library/results.js'
import { itemFiles, loadViewerFiles, viewerFile, viewerIdAt, viewerPosition, viewerResults } from './session.js'
import { V, panTo, shownId, closeViewer, fitShare, flip, gesturePercent, isPicture, isPlayer, isText, isVideo, liveClip, oneToOne, setTextZoom, setViewerMounted, step, textPercentFrom, textView, viewerItem, wheelFactor, zoomAround, zoomFromPercent } from './viewerControls.js'
import { playUrl } from '../platform/fileUrls.js'
import { call } from '../platform/ipc.js'
import ItemPreview from '../formats/ItemPreview.vue'
import ItemViewerStrip from './ItemViewerStrip.vue'
import ItemMenu from '../library/ItemMenu.vue'
import { shortcutLabel } from '../app/shortcuts.js'
import { preloadAround } from '../formats/imagePool.js'
import { copyFrame, loadSubtitles, saveFrame, setFrameAsThumbnail } from '../formats/playerState.js'
import { t } from '../shared/i18n.js'

// Mouse controls:
//   wheel            → previous / next item  (Settings: "wheel zooms" swaps these)
//   Ctrl + wheel     → zoom at the pointer
//   right-drag up or down → zoom at where the button went down
//   left-drag → pan;  double-click → close;  middle-click → open with default app
// Over text the wheel scrolls, Ctrl + wheel and the right-button gesture change its size, and left-drag and double-click select.
// opening the viewer starts as new; stepping to another item keeps the zoom when Settings ask for it
const startView = (opening) => {
  V.rotation = 0
  V.live = false
  V.alt = false
  V.flip = false
  if (S.ui.rememberZoom && !opening) return
  V.zoom = 1
  V.pan = { x: 0, y: 0 }
  // on opening, the stage is measured first, in onMounted
  if (S.ui.defaultImageSize === 'original' && !opening) nextTick(oneToOne)
}
startView(true)
watch([() => S.viewer?.id, () => S.viewer?.file ?? null], () => startView(false))
// an item on show counts as opened, for Recently used
watch(() => S.viewer?.id, (id) => id != null && call('mark_used', { id }).catch(() => {}), { immediate: true })
// the files grouped into the item, again whenever its row changes (a file grouped in or split off)
watch(
  [() => S.viewer?.id, () => viewerItem.value],
  ([id, row]) => {
    if (id != null && row) loadViewerFiles(id, row.parts)
  },
  { immediate: true },
)
// once this item has had a moment, its neighbours are decoded so stepping to them is instant
const PRELOAD_AFTER_MS = 120
let preloadTimer = null
function schedulePreload() {
  clearTimeout(preloadTimer)
  preloadTimer = setTimeout(() => {
    const el = stage.value
    if (!S.viewer || !el) return
    const { pos } = viewerPosition()
    ensureItems([-1, 1, 2].map((d) => viewerIdAt(pos + d)).filter((x) => x != null))
    // the neighbours' sizes may still be on their way
    setTimeout(() => {
      if (!S.viewer) return
      preloadAround(viewerPosition().pos, el.clientWidth, el.clientHeight, viewerResults().ids)
      const files = itemFiles()
      if (files.length) preloadAround(files.indexOf(viewerFile()), el.clientWidth, el.clientHeight, files)
    }, 30)
  }, PRELOAD_AFTER_MS)
}
watch([() => S.viewer?.index, viewerFile, () => itemFiles().length], schedulePreload, { immediate: true })
onBeforeUnmount(() => clearTimeout(preloadTimer))
const stage = ref()
const menu = ref()

let ro
const measure = () => {
  if (!stage.value) return
  V.stageW = stage.value.clientWidth
  V.stageH = stage.value.clientHeight
}
onMounted(() => {
  ro = new ResizeObserver(measure)
  ro.observe(stage.value)
  measure()
  setViewerMounted(true)
  if (S.ui.defaultImageSize === 'original') nextTick(oneToOne)
})
onBeforeUnmount(() => {
  ro?.disconnect()
  setViewerMounted(false)
})
// on a video the menu starts with what can be done with the frame on show
const mediaItems = () =>
  isVideo.value
    ? [
        { label: t('viewer.menu.frameThumbnail'), icon: 'photo-star', command: setFrameAsThumbnail },
        { label: t('viewer.menu.copyFrame'), icon: 'copy', shortcut: shortcutLabel(S.ui.keys, 'player.copyFrame'), command: copyFrame },
        { label: t('viewer.menu.saveFrame'), icon: 'device-floppy', shortcut: shortcutLabel(S.ui.keys, 'player.saveFrame'), visible: !S.items.get(viewerFile())?.read_only, command: saveFrame },
        { label: t('viewer.menu.loadSubtitles'), icon: 'badge-cc', command: loadSubtitles },
        { separator: true },
      ]
    : []
const WHEEL_SEEK_SECONDS = 5
const WHEEL_VOLUME_STEP = 0.1
// Full screen is the viewer over the whole screen with its bar hidden; the picture fits again on the way in and out.
watch(() => S.fullscreen, (on) => {
  getCurrentWindow().setFullscreen(!!on).catch(() => {})
  V.zoom = 1
  V.pan = { x: 0, y: 0 }
})
onBeforeUnmount(() => {
  if (S.fullscreen) S.fullscreen = false
})
const WHEEL_PAGING_MS = 70
let pagingTimer = null
onBeforeUnmount(() => clearTimeout(pagingTimer))
/** A point on screen, measured from the middle of the stage. */
function fromMiddle(e) {
  const r = stage.value.getBoundingClientRect()
  return [e.clientX - r.left - r.width / 2, e.clientY - r.top - r.height / 2]
}
function wheel(e) {
  if (isText.value) {
    if (e.ctrlKey) {
      e.preventDefault()
      setTextZoom(textView.value.zoom * wheelFactor(e.deltaY))
    }
    return
  }
  e.preventDefault()
  const zoomKey = e.ctrlKey && isPicture.value
  const mode = S.ui.viewerWheel || 'navigate'
  // over a video the wheel seeks, changes volume or pages through items, whatever it does over a picture
  const video = !zoomKey && stage.value?.querySelector('video, audio')
  const pageVideo = video && S.ui.videoWheel === 'navigate'
  if (video && !pageVideo && mode !== 'navigate') {
    if (S.ui.videoWheel === 'volume') video.volume = Math.max(0, Math.min(1, video.volume - Math.sign(e.deltaY) * WHEEL_VOLUME_STEP))
    else video.currentTime = Math.max(0, video.currentTime + Math.sign(e.deltaY) * WHEEL_SEEK_SECONDS)
    return
  }
  if (e.ctrlKey && !isPicture.value) return
  if (pageVideo) {
    clearTimeout(pagingTimer)
    pagingTimer = setTimeout(() => step(e.deltaY > 0 ? 1 : -1), WHEEL_PAGING_MS)
    return
  }
  if (zoomKey || (mode === 'zoom' && isPicture.value)) {
    const [px, py] = fromMiddle(e)
    zoomAround(V.zoom * wheelFactor(e.deltaY), px, py)
    return
  }
  if (mode === 'scroll') {
    panTo({ x: V.pan.x - e.deltaX, y: V.pan.y - e.deltaY })
    return
  }
  // the last notch of a burst decides
  clearTimeout(pagingTimer)
  const dir = e.deltaY > 0 || (e.deltaY === 0 && e.deltaX > 0) ? 1 : -1
  pagingTimer = setTimeout(() => step(dir), WHEEL_PAGING_MS)
}

// the right-button gesture: up or down zooms, no movement opens the menu
const GESTURE_STILL_PX = 3
let rightDragged = false
// a gesture whose button came up outside the window never saw its pointerup, and ends when the next one starts
let endGesture = null
onBeforeUnmount(() => endGesture?.())
function context(e) {
  if (rightDragged) return
  if (S.viewer) menu.value.show(e)
}
function rightGesture(e) {
  const sx = e.clientX
  const sy = e.clientY
  const [px, py] = fromMiddle(e)
  const text = isText.value
  const percent0 = text ? textView.value.zoom * 100 : V.zoom * fitShare.value * 100
  let zooming = false
  const move = (ev) => {
    const dx = ev.clientX - sx
    const dy = sy - ev.clientY
    if (Math.max(Math.abs(dx), Math.abs(dy)) >= GESTURE_STILL_PX) rightDragged = true
    if (!zooming && Math.abs(dy) <= Math.abs(dx)) return
    zooming = true
    if (text) return setTextZoom(textPercentFrom(gesturePercent(percent0, dy)) / 100)
    // from where the last move left the picture: one anchored at the start would pull back towards it once an edge stops holding it
    zoomAround(zoomFromPercent(gesturePercent(percent0, dy)), px, py)
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    window.removeEventListener('pointercancel', up)
    endGesture = null
  }
  endGesture?.()
  endGesture = up
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
  window.addEventListener('pointercancel', up)
}
function down(e) {
  if (e.button === 1) {
    openItem(shownId(), 'open')
    return
  }
  if (!isPicture.value && !isText.value) return
  rightDragged = false
  if (e.button === 2) {
    if (S.ui.rightDragZoom !== false) rightGesture(e)
    return
  }
  if (!isPicture.value) return
  const sx = e.clientX
  const sy = e.clientY
  const pan0 = { ...V.pan }
  const move = (ev) => panTo({ x: pan0.x + ev.clientX - sx, y: pan0.y + ev.clientY - sy })
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
defineExpose({ close: closeViewer })
</script>

<template>
  <div class="viewer" :class="['strip-' + (S.ui.viewerStripSide || 'bottom'), { show: S.fullscreen, pixelated: S.ui.pixelated, gray: L().grayscale, checker: S.ui.transparency }]">
    <div ref="stage" class="stage" :class="{ player: isPlayer }" @wheel="wheel" @pointerdown="down" @contextmenu.prevent="context" @dblclick="!isText && closeViewer()">
      <ItemPreview :id="shownId()" :zoom="V.zoom" :pan="V.pan" :rotation="V.rotation" :flip="V.flip" />
      <video v-if="V.live && liveClip" class="live-clip" :src="playUrl(liveClip)" autoplay :muted="S.ui.hoverMuted !== false" @ended="V.live = false" @pointerdown.stop @dblclick.stop="V.live = false" />
    </div>
    <ItemMenu ref="menu" :top="mediaItems" />
    <ItemViewerStrip v-if="S.ui.viewerStrip && !S.fullscreen && !S.itemWindow" :side="S.ui.viewerStripSide || 'bottom'" />
  </div>
</template>

<style scoped>
.viewer { position: absolute; inset: 0; display: flex; flex-direction: column; }
/* every kind of item fills the stage to the window's edges */
.stage { flex: 1; min-height: 0; min-width: 0; position: relative; cursor: grab; }
.viewer.strip-left, .viewer.strip-right { flex-direction: row; }
.viewer.strip-left > :deep(.viewer-strip) { order: -1; }
.viewer .stage.player { cursor: default; }
.live-clip { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; z-index: 1; background: transparent; }
.viewer .stage:has(.viewer-page), .viewer .stage:has(.fills-stage) { cursor: default; }
.viewer.show { position: fixed; z-index: 1000; background: #000; }
.viewer.pixelated :deep(img) { image-rendering: pixelated; }
.viewer.gray :deep(img), .viewer.gray :deep(video) { filter: grayscale(1); }
/* only behind a picture that has loaded, where its transparent parts show it; never as a stand-in */
.viewer.checker :deep(.holder:not(:empty)) { background: conic-gradient(#3a3b3e 25%, #2c2d30 0 50%, #3a3b3e 0 75%, #2c2d30 0) 0 0 / 24px 24px; }
</style>
