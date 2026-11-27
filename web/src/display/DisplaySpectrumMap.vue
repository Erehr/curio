<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { spectrumView as V } from './spectrumView.js'

// The whole Spectrum view small, each picture as its main colour, with the part on screen marked; a click or a drag goes there.
const box = ref()
const canvas = ref()
const width = ref(0)
const PAD = 4

function frame() {
  const A = V.layout
  const spanX = A.max[0] - A.min[0] || 1
  const spanY = A.max[1] - A.min[1] + 1
  const k = Math.min((width.value - PAD * 2) / spanX, (width.value - PAD * 2) / spanY)
  return { k, ox: width.value / 2 - ((A.min[0] + A.max[0]) / 2) * k, oy: width.value / 2 - ((A.min[1] + A.max[1]) / 2) * k }
}
function paint() {
  const cv = canvas.value
  const A = V.layout
  if (!cv || !A || !width.value) return
  const dpr = window.devicePixelRatio || 1
  cv.width = cv.height = Math.round(width.value * dpr)
  const g = cv.getContext('2d')
  g.setTransform(dpr, 0, 0, dpr, 0, 0)
  g.clearRect(0, 0, width.value, width.value)
  const { k, ox, oy } = frame()
  const cs = getComputedStyle(cv)
  if (A.folders?.count) {
    const F = A.folders
    g.fillStyle = cs.getPropertyValue('--text-faint').trim() || '#888'
    for (let i = 0; i < F.count; i++) {
      g.fillRect(ox + F.x[i] * k, oy + (F.row[i] - 0.5) * k, Math.max(1.5, F.w * k), Math.max(1.5, (1 - A.gap) * k))
    }
  }
  const rgb = V.rgb
  for (let j = 0; j < A.n; j++) {
    g.fillStyle = `rgb(${rgb[j * 3]},${rgb[j * 3 + 1]},${rgb[j * 3 + 2]})`
    g.fillRect(ox + A.x[j] * k, oy + (A.row[j] - 0.5) * k, Math.max(1, A.w[j] * k), Math.max(1, (1 - A.gap) * k))
  }
  g.strokeStyle = cs.getPropertyValue('--text').trim() || '#fff'
  g.lineWidth = 1.5
  g.strokeRect(ox + (V.x - V.w / 2) * k, oy + (V.y - V.h / 2) * k, V.w * k, V.h * k)
}
let raf = 0
watch(() => [V.layout, V.x, V.y, V.w, V.h, width.value], () => {
  if (!raf) raf = requestAnimationFrame(() => ((raf = 0), paint()))
}, { immediate: true })

function down(e) {
  if (e.button !== 0 || !V.layout) return
  const go = (ev) => {
    const r = canvas.value.getBoundingClientRect()
    const { k, ox, oy } = frame()
    V.x = (ev.clientX - r.left - ox) / k
    V.y = (ev.clientY - r.top - oy) / k
  }
  go(e)
  const up = () => {
    window.removeEventListener('pointermove', go)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', go)
  window.addEventListener('pointerup', up)
}

let ro
onMounted(() => {
  ro = new ResizeObserver(() => (width.value = box.value.clientWidth))
  ro.observe(box.value)
  width.value = box.value.clientWidth
})
onBeforeUnmount(() => {
  ro?.disconnect()
  cancelAnimationFrame(raf)
})
</script>

<template>
  <div ref="box" class="spectrum-map">
    <canvas v-if="V.layout?.n || V.layout?.folders?.count" ref="canvas" :style="{ width: width + 'px', height: width + 'px' }" :aria-label="$t('grid.spectrum.map')" @pointerdown="down" />
    <div v-if="V.missing" class="note faint">{{ $t('grid.spectrum.missing', { count: V.missing.toLocaleString() }, V.missing) }}</div>
  </div>
</template>

<style scoped>
.spectrum-map { margin-top: calc(0.84rem * var(--g)); }
.note { font-size: 0.85rem; margin-top: calc(0.42rem * var(--g)); }
.spectrum-map canvas { display: block; cursor: crosshair; border-radius: calc(8px * var(--r)); background: color-mix(in srgb, var(--text) 5%, transparent); touch-action: none; }
</style>
