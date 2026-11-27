<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

// A sound's loudness, played part in the accent colour and the rest in grey.
const props = defineProps({ peaks: Object, played: { type: Number, default: 0 } })
const canvas = ref()
const GREY = '#6b6c70'
// quiet stretches stay visible as a thin line
const MIN_BAR = 1

function colour(name, fallback) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback
}
function draw() {
  const c = canvas.value
  if (!c) return
  const dpr = window.devicePixelRatio || 1
  const w = Math.max(1, Math.round(c.clientWidth * dpr))
  const h = Math.max(1, Math.round(c.clientHeight * dpr))
  if (c.width !== w || c.height !== h) {
    c.width = w
    c.height = h
  }
  const g = c.getContext('2d')
  g.clearRect(0, 0, w, h)
  const p = props.peaks
  if (!p?.length) return
  const mid = h / 2
  const cut = Math.round(props.played * w)
  const accent = colour('--accent', '#3b9dff')
  for (let x = 0; x < w; x++) {
    const v = p[Math.min(p.length - 1, Math.floor((x / w) * p.length))] / 255
    const half = Math.max(MIN_BAR, v * mid)
    g.fillStyle = x < cut ? accent : GREY
    g.fillRect(x, mid - half, 1, half * 2)
  }
}
let ro
onMounted(() => {
  ro = new ResizeObserver(draw)
  ro.observe(canvas.value)
  draw()
})
onBeforeUnmount(() => ro?.disconnect())
watch(() => [props.peaks, props.played], draw)
</script>

<template>
  <canvas ref="canvas" class="wave" />
</template>

<style scoped>
.wave { display: block; width: 100%; height: 100%; }
</style>
