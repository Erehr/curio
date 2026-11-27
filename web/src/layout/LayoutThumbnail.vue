<script setup>
import { computed } from 'vue'
import { thumbRects } from './layouts.js'

// A small picture of a window arranged by a layout, drawn from its own settings: panels as solid blocks with a gap between them, so where each one sits reads at a glance.
// With a theme (`palette`, its tokens, and `dark`) it shows that theme instead: its colours, lines of text in a panel and an accent button over the grid.
const props = defineProps({ layout: Object, aspect: { type: String, default: 'horizontal' }, edge: Boolean, palette: Object, dark: Boolean })
const GAP = 3
const rects = computed(() => {
  if (!props.layout) return []
  const out = thumbRects(props.layout, props.aspect, { edgeReveal: props.edge })
  if (!props.palette) return out
  const grid = out.find((r) => r.kind === 'grid')
  const lib = out.find((r) => r.kind === 'library')
  if (lib) for (const [i, w] of [0.6, 0.4, 0.5].entries()) out.push({ kind: 'line', x: lib.x + lib.w * 0.18, y: 16 + i * 10, w: lib.w * w, h: 4 })
  if (grid) out.push({ kind: 'button', x: grid.x + grid.w * 0.08, y: grid.y + grid.h * 0.14, w: grid.w * 0.34, h: 12 })
  return out
})
// marks sit inside their panel, so only blocks get the gap
const box = (r) => {
  const g = ['tab', 'edge', 'line', 'button'].includes(r.kind) ? 0 : GAP / 2
  return { left: `calc(${r.x}% + ${g}px)`, top: `calc(${r.y}% + ${g}px)`, width: `calc(${r.w}% - ${2 * g}px)`, height: `calc(${r.h}% - ${2 * g}px)` }
}
// a light theme's panels are only a shade off its background, so they are darkened a little to stand out as they do in the window
const vars = computed(() => {
  const p = props.palette
  if (!p) return {}
  const panel = props.dark ? p['surface-2'] : `color-mix(in srgb, ${p.text} 10%, ${p.panel})`
  return { '--lt-win': p.bg, '--lt-panel': panel, '--lt-bar': `color-mix(in srgb, ${p.text} 24%, ${p.bg})`, '--lt-bar2': `color-mix(in srgb, ${p.text} 12%, ${p.bg})`, '--lt-line': p['text-muted'], '--lt-accent': p.accent, '--lt-edge-line': `color-mix(in srgb, ${p.text} 14%, transparent)` }
})
</script>

<template>
  <span class="lthumb" :class="[aspect, { themed: !!palette }]" :style="vars">
    <span class="lt-in"><span v-for="(r, i) in rects" :key="i" class="lt" :class="['lt-' + r.kind, { active: r.active }]" :style="box(r)" /></span>
  </span>
</template>

<style>
.lthumb { --lt-win: color-mix(in srgb, var(--text) 12%, var(--bg)); --lt-panel: color-mix(in srgb, var(--text) 72%, var(--bg)); --lt-bar: color-mix(in srgb, var(--text) 45%, var(--bg)); --lt-bar2: color-mix(in srgb, var(--text) 28%, var(--bg)); --lt-accent: var(--accent); position: relative; display: block; height: 100%; aspect-ratio: 16 / 10; overflow: hidden; border-radius: calc(6px * var(--r)); background: var(--lt-win); }
.lthumb.vertical { aspect-ratio: 10 / 16; }
.lthumb.themed { box-shadow: inset 0 0 0 1px var(--lt-edge-line); }
.lt-in { position: absolute; inset: 0.21rem; }
.lt { position: absolute; box-sizing: border-box; border-radius: calc(3px * var(--r)); }
.lt-library, .lt-inspector, .lt-rail { background: var(--lt-panel); }
.lt-toolbar { background: var(--lt-bar); border-radius: calc(2px * var(--r)); }
.lt-filterbar { background: var(--lt-bar2); border-radius: calc(2px * var(--r)); }
.lt-grid { background: transparent; }
.lt-edge { border: 1.5px dashed var(--lt-accent); border-radius: calc(2px * var(--r)); }
.lt-tab { background: color-mix(in srgb, var(--text) 40%, var(--bg)); border-radius: calc(2px * var(--r)); }
.lt-tab.active { background: var(--lt-accent); }
.lt-line { background: var(--lt-line); border-radius: calc(2px * var(--r)); opacity: 0.8; }
.lt-button { background: var(--lt-accent); border-radius: calc(4px * var(--r)); }
</style>
