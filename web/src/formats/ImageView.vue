<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { fitBox, imageUrlFor, neededPixels, thumbOf } from './index.js'
import { loadImage, readyImage } from './imagePool.js'

// A picture in three layers of quality that all occupy exactly the same box, so nothing jumps:
//   1. the thumbnail, stretched to the final size, on screen at once;
//   2. a copy made to measure for this window, swapped in once it is decoded;
//   3. a larger copy only when zooming asks for more pixels than are on screen.
const props = defineProps({ item: Object, zoom: { type: Number, default: 1 }, pan: { type: Object, default: () => ({ x: 0, y: 0 }) }, compact: Boolean, flip: Boolean })
const emit = defineEmits(['failed'])
const area = ref()
const size = ref({ w: 0, h: 0 })
const holder = ref()
const natural = ref(null)
const sharp = ref(false)
// a short wait while stepping quickly, so pictures flown past aren't decoded for nothing
const SETTLE_MS = 50
const ZOOM_SETTLE_MS = 160
// zooming in past what is shown asks at once for twice what it needs yet, so the sharper picture is on its way while the zoom goes on
const ZOOM_LEAD = 2

// turned only here, in the full view; the thumbnail and the small preview stay as the file is
const rotation = computed(() => (props.compact ? 0 : props.item?.rotation || 0))
const box = computed(() => fitBox(props.item, size.value.w, size.value.h, natural.value, rotation.value))
const thumb = computed(() => thumbOf(props.item))
const style = computed(() => {
  const b = box.value
  const t = props.compact ? '' : `translate(${props.pan.x}px, ${props.pan.y}px) scale(${props.zoom}) rotate(${rotation.value}deg)${props.flip ? ' scaleX(-1)' : ''}`
  return b ? { width: b.w + 'px', height: b.h + 'px', transform: t } : { width: '100%', height: '100%', transform: t }
})

let shownUrl = null
let shownPx = 0
// the picture asked for and not here yet
let pending = null
let token = 0
let timer = null
// the long edge an address was asked at; the file itself has all there is
const pxOf = (url) => Number(/\/p\/\d+\/(\d+)/.exec(url)?.[1] ?? Infinity)
function show(img, url) {
  shownUrl = url
  shownPx = pxOf(url)
  if (pending && pending.px <= shownPx) pending = null
  sharp.value = true
  const el = holder.value
  if (!el) return
  // pool entries are shared by URL: clone when another ImageView already holds the element so it is never stolen from the viewer
  const node = img.parentNode && img.parentNode !== el ? img.cloneNode(false) : img
  node.className = 'sharp-img'
  node.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;object-fit:contain;display:block'
  el.replaceChildren(node)
  // a picture whose size wasn't known yet now gets its real box
  if (!props.item?.width && img.naturalWidth) natural.value = { w: img.naturalWidth, h: img.naturalHeight }
}
function want(wait, lead = 1) {
  const it = props.item
  if (!it) return clearTimeout(timer)
  const b = box.value
  const need = b ? neededPixels(b, props.compact ? 1 : props.zoom) : Math.max(size.value.w, size.value.h, 1024)
  // sharp enough already, or about to be: zooming out and small changes ask for nothing
  if ((shownUrl && shownPx >= need) || (pending && pending.px >= need)) return
  clearTimeout(timer)
  const url = imageUrlFor(it, need * lead)
  if (url === shownUrl) return
  const ready = readyImage(url)
  if (ready) return show(ready, url)
  // the token changes with the item, not with each request: a picture that arrives after a larger one was asked for still shows meanwhile
  const mine = token
  const px = pxOf(url)
  pending = { px }
  timer = setTimeout(() => {
    loadImage(url)
      .then((img) => {
        if (mine === token && px > shownPx) show(img, url)
      })
      .catch(() => {
        if (mine !== token || pending?.px !== px) return
        pending = null
        emit('failed')
      })
  }, wait)
}
watch(() => [props.item?.id, props.item?.thumb_ver], ([id], old) => {
  // another item, or the file itself changed: the old picture goes at once and the thumbnail stands in
  if (id !== old?.[0] || sharp.value) {
    shownUrl = null
    shownPx = 0
    pending = null
    clearTimeout(timer)
    sharp.value = false
    natural.value = null
    token++
    holder.value?.replaceChildren()
  }
  want(SETTLE_MS)
})
watch(box, (b, was) => {
  if (b && (!was || b.w !== was.w || b.h !== was.h)) want(shownUrl ? ZOOM_SETTLE_MS : 0)
})
watch(() => props.zoom, (z, was) => (z > was ? want(0, ZOOM_LEAD) : want(ZOOM_SETTLE_MS)))

let ro
onMounted(() => {
  ro = new ResizeObserver(() => {
    const el = area.value
    if (el) size.value = { w: el.clientWidth, h: el.clientHeight }
  })
  ro.observe(area.value)
  size.value = { w: area.value.clientWidth, h: area.value.clientHeight }
  want(0)
})
onBeforeUnmount(() => {
  ro?.disconnect()
  clearTimeout(timer)
  token++
})
</script>

<template>
  <div ref="area" class="image-area">
    <div class="image-box" :class="{ round: compact }" :style="style">
      <img v-if="thumb" :src="thumb" class="stand-in" :class="{ gone: sharp }" draggable="false" />
      <div ref="holder" class="holder" />
    </div>
  </div>
</template>

<style scoped>
.image-area { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; }
.image-box { position: relative; flex: none; transform-origin: center; user-select: none; }
.image-box.round { border-radius: calc(6px * var(--r)); overflow: hidden; }
.stand-in { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; display: block; }
/* see-through pictures must not show the thumbnail behind them */
.stand-in.gone { visibility: hidden; }
.holder { position: absolute; inset: 0; }
</style>
