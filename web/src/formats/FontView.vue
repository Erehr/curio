<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { call } from '../platform/ipc.js'
import { t } from '../shared/i18n.js'
import AppIcon from '../shared/AppIcon.vue'
import { describeFont, faceBytes, glyphNames, loadFace } from './fontFile.js'
import { RTL, charList, codePoint, sampleFor } from './fontSamples.js'

// The font viewer: the font's sample text, a waterfall of sizes, every character it has, and what it says of itself. Its views, size and look are Curio's toolbar widgets, the Fonts entry's `viewer_tools`.
const props = defineProps({ item: { type: Object, required: true } })

const PLUGIN = 'builtin.format.font'
const VIEWS = ['sample', 'waterfall', 'glyphs', 'details']
const VIEW_ICONS = { sample: 'align-left', waterfall: 'text-size', glyphs: 'layout-grid', details: 'info-circle' }
// backgrounds and the text drawn on them; the theme's own follows it
const LOOKS = {
  theme: { background: 'var(--bg)', foreground: 'var(--text)' },
  paper: { background: '#f7f6f3', foreground: '#1d1d1f' },
  ink: { background: '#1d1d1f', foreground: '#f7f6f3' },
  blue: { background: '#dbe5f3', foreground: '#13243d' },
  yellow: { background: '#f5eec8', foreground: '#3a3006' },
}
const WATERFALL = [120, 96, 72, 60, 48, 42, 36, 32, 28, 24, 21, 18, 16, 14, 12, 10]
const SIZE = { min: 12, max: 160 }

const prefs = computed(() => ({ view: 'sample', size: 40, look: 'theme', ...S.ui.fontViewer }))
const setPref = (k, v) => {
  S.ui.fontViewer = { ...prefs.value, [k]: v }
  saveUi()
}

/** The font shown now: its description, face and the name it is loaded under. */
const cur = ref(null)
const problem = ref('')
const axes = reactive({})
let loading = 0

async function show(face = 0) {
  const n = ++loading
  const id = props.item.id
  try {
    const info = await describeFont(id, face)
    const { family, unload } = await loadFace(await faceBytes(id, info, face))
    if (n !== loading) return unload()
    cur.value?.unload()
    cur.value = { id, info, face, family, unload }
    for (const k of Object.keys(axes)) delete axes[k]
    picked.value = null
    problem.value = ''
  } catch (e) {
    if (n !== loading) return
    cur.value?.unload()
    cur.value = null
    problem.value = t('formats.font.problem', { error: e?.message || String(e) })
  }
}
watch(() => `${props.item.id}:${props.item.mtime}:${props.item.size}`, () => show(0), { immediate: true })
onBeforeUnmount(() => {
  loading++
  cur.value?.unload()
  if (S.viewerTools?.frame === 'font') S.viewerTools = null
})

const look = computed(() => (prefs.value.view !== 'details' && LOOKS[prefs.value.look]) || LOOKS.theme)
const rtl = computed(() => RTL.has(cur.value?.info.primary))
/** How the font is written at `px`, with the variable axes as set. */
const fontStyle = (px) => {
  const settings = Object.entries(axes).map(([tag, v]) => `"${tag}" ${v}`).join(', ')
  return { fontFamily: `"${cur.value.family}", var(--ui-font)`, fontSize: px ? `${px}px` : undefined, fontVariationSettings: settings || 'normal' }
}

// the line written in the font: the window's sample text while one is set, else the font's own
const sample = computed(() => (cur.value ? sampleFor(cur.value.info) : null))
const lineText = computed(() => S.ui.fontSample?.text || sample.value?.line || '')
const lineEl = ref()
// typed here, it becomes the window's sample text, which the list's Font sample column writes too
function typed(e) {
  S.ui.fontSample = { text: e.target.textContent }
  saveUi()
}
// the line being typed in is left as it is; set elsewhere, it shows here
watch(lineText, (v) => {
  if (lineEl.value && document.activeElement !== lineEl.value && lineEl.value.textContent !== v) lineEl.value.textContent = v
})
watch(lineEl, (el) => el && (el.textContent = lineText.value))

// ------------------------------------------------------------ glyphs
const picked = ref(null)
const pickedName = ref('')
const grid = ref()
const scroll = ref(0)
const gridSize = reactive({ w: 0, h: 0 })
const chars = computed(() => (cur.value ? charList(cur.value.info.ranges) : []))
const cell = computed(() => Math.round(Math.max(44, prefs.value.size * 1.5)))
const cols = computed(() => Math.max(1, Math.floor(gridSize.w / cell.value)))
const rows = computed(() => Math.ceil(chars.value.length / cols.value))
/** Only the rows in view are drawn: a CJK font has tens of thousands of characters. */
const cells = computed(() => {
  const out = []
  const first = Math.max(0, Math.floor(scroll.value / cell.value) - 2)
  const last = Math.min(rows.value, Math.ceil((scroll.value + gridSize.h) / cell.value) + 2)
  for (let r = first; r < last; r++) {
    for (let c = 0; c < cols.value; c++) {
      const i = r * cols.value + c
      if (i >= chars.value.length) break
      out.push({ cp: chars.value[i], left: c * cell.value, top: r * cell.value })
    }
  }
  return out
})
async function pick(cp) {
  picked.value = cp
  pickedName.value = ''
  const shown = cur.value
  const names = await glyphNames(shown.id, shown.face, [cp]).catch(() => [null])
  if (cur.value === shown && picked.value === cp) pickedName.value = names[0] ? t('formats.font.glyphName', { name: names[0] }) : ''
}
const copyGlyph = () => navigator.clipboard.writeText(String.fromCodePoint(picked.value)).catch(() => {})
let ro = null
watch(grid, (el) => {
  ro?.disconnect()
  if (!el) return
  ro = new ResizeObserver(() => Object.assign(gridSize, { w: el.clientWidth, h: el.clientHeight }))
  ro.observe(el)
  scroll.value = el.scrollTop
})
watch([() => prefs.value.view, cur], () => {
  if (prefs.value.view === 'glyphs' && cur.value && picked.value === null) pick(chars.value.find((cp) => cp > 0x20) ?? chars.value[0] ?? 0x41)
}, { immediate: true })
onBeforeUnmount(() => ro?.disconnect())

// ------------------------------------------------------------ details
const detailRows = computed(() => {
  const i = cur.value?.info
  if (!i) return []
  const n = i.names || {}
  const d = (k) => t(`formats.font.details.${k}`)
  const axesText = (i.axes || []).filter((a) => !a.hidden).map((a) => t('formats.font.details.axisRange', { name: a.name, min: +a.min.toFixed(2), max: +a.max.toFixed(2) })).join(', ')
  const webHref = (u) => (typeof u === 'string' && /^https?:\/\//i.test(u.trim()) ? u.trim() : null)
  return [
    { label: d('family'), value: i.family },
    { label: d('style'), value: i.style },
    { label: d('fullName'), value: n.full },
    { label: d('postscriptName'), value: n.postscript, mono: true },
    { label: d('version'), value: i.version },
    { label: d('designer'), value: n.designer, href: webHref(n.designerUrl) },
    { label: d('foundry'), value: n.foundry, href: webHref(n.vendorUrl) },
    { label: d('license'), value: i.license, href: webHref(n.licenseUrl) },
    { label: d('copyright'), value: n.copyright },
    { label: d('trademark'), value: n.trademark },
    { label: d('format'), value: i.format },
    { label: d('outlines'), value: i.outlines },
    { label: d('faces'), value: i.faces > 1 ? i.faces.toLocaleString() : null, mono: true },
    { label: d('glyphs'), value: i.glyphs.toLocaleString(), mono: true },
    { label: d('characters'), value: i.characters.toLocaleString(), mono: true },
    { label: d('scripts'), value: i.scripts.join(', ') },
    { label: d('category'), value: i.category },
    { label: d('weight'), value: String(i.weight), mono: true },
    { label: d('width'), value: i.width },
    { label: d('italic'), value: i.italic ? d('yes') : d('no') },
    { label: d('variableAxes'), value: axesText || null },
    { label: d('namedStyles'), value: i.instances?.length ? i.instances.map((x) => x.name).join(', ') : null },
    { label: d('features'), value: i.features.length ? i.features.join(' ') : null, mono: true },
    { label: d('colorGlyphs'), value: i.color.length ? i.color.join(', ') : null },
    { label: d('embedding'), value: i.embedding },
    { label: d('vendorId'), value: i.vendor, mono: true },
    { label: d('unitsPerEm'), value: String(i.unitsPerEm), mono: true },
  ].filter((r) => r.value)
})
const longTexts = computed(() => {
  const n = cur.value?.info.names || {}
  return [[t('formats.font.details.description'), n.description], [t('formats.font.details.licenseText'), n.license]].filter(([, text]) => text)
})
const openUrl = (url) => call('open_url', { url }).catch(() => {})

// ------------------------------------------------------------ the toolbar
function onTool(id) {
  if (VIEWS.includes(id)) setPref('view', id)
}
function onChange(id, value) {
  if (!cur.value || value == null) return
  if (id === 'size') return setPref('size', Math.min(SIZE.max, Math.max(SIZE.min, Number(value))))
  if (id === 'look' && value in LOOKS) return setPref('look', value)
  if (id === 'face') return show(Number(value))
  if (id === 'instance') {
    const inst = cur.value.info.instances[Number(value)]
    cur.value.info.axes.forEach((a, k) => (axes[a.tag] = inst?.coords[k] ?? a.default))
    return
  }
  if (String(id).startsWith('axis:')) axes[id.slice(5)] = Number(value)
}
/** The buttons, slider and panels of Curio's toolbar widgets for this font, as a viewer page would give them with `viewer.setTools`. */
const tools = computed(() => {
  const info = cur.value?.info
  if (!info) return null
  const p = prefs.value
  const lookPanel = [{ type: 'swatches', id: 'look', label: t('formats.font.look.background'), value: p.look, options: Object.entries(LOOKS).map(([value, l]) => ({ value, label: t(`formats.font.looks.${value}`), background: l.background, foreground: l.foreground, icon: 'letter-a' })) }]
  const shown = (info.axes || []).filter((a) => !a.hidden)
  if (shown.length) {
    lookPanel.push({ type: 'section', label: t('formats.font.look.axes') })
    if (info.instances?.length) lookPanel.push({ type: 'select', id: 'instance', label: t('formats.font.look.namedStyle'), value: null, options: info.instances.map((x, i) => ({ value: i, label: x.name })) })
    for (const a of shown) lookPanel.push({ type: 'slider', id: `axis:${a.tag}`, label: a.name, min: a.min, max: a.max, step: a.max - a.min > 20 ? 1 : 0.1, value: axes[a.tag] ?? a.default })
  }
  const buttons = VIEWS.map((id) => ({ id, label: t(`formats.font.views.${id}`), icon: VIEW_ICONS[id], on: p.view === id, panel: null }))
  if (info.faces > 1) buttons.push({ id: 'face', label: t('formats.font.look.face'), icon: 'stack-2', on: false, panel: [{ label: '', icon: null, controls: [{ type: 'select', id: 'face', label: t('formats.font.look.face'), value: cur.value.face, options: info.faceNames.map((label, value) => ({ value, label })) }] }] })
  buttons.push({ id: 'look', label: t('formats.font.look.appearance'), icon: 'palette', on: false, panel: [{ label: '', icon: null, controls: lookPanel }] })
  const slider = ['sample', 'glyphs'].includes(p.view) ? { id: 'size', label: t('formats.font.look.size'), min: SIZE.min, max: SIZE.max, step: 1, value: p.size, dir: 'ltr' } : null
  return { frame: 'font', plugin: PLUGIN, labels: [], buttons, slider, keys: VIEWS, onTool, onChange, onKey: onTool }
})
watch(tools, (v) => (S.viewerTools = v), { immediate: true })
onMounted(() => nextTick(() => grid.value && (scroll.value = grid.value.scrollTop)))
</script>

<template>
  <div class="font-view" :style="{ '--page-bg': look.background, '--page-fg': look.foreground }">
    <div v-if="problem" class="problem">{{ problem }}</div>
    <div v-else-if="cur" class="view" :class="prefs.view">
      <template v-if="prefs.view === 'sample'">
        <div class="name" dir="auto">{{ [cur.info.family, cur.info.style].filter(Boolean).join(' ') }}</div>
        <div class="title" :dir="rtl ? 'rtl' : undefined" contenteditable="plaintext-only" spellcheck="false" :style="fontStyle(prefs.size * 1.8)">{{ sample.title }}</div>
        <div ref="lineEl" class="line" :dir="rtl ? 'rtl' : undefined" contenteditable="plaintext-only" spellcheck="false" :style="fontStyle(prefs.size)" @input="typed"></div>
        <div class="body" :dir="rtl ? 'rtl' : undefined" contenteditable="plaintext-only" spellcheck="false" :style="fontStyle(Math.max(14, prefs.size * 0.5))">{{ sample.body }}</div>
      </template>
      <template v-else-if="prefs.view === 'waterfall'">
        <div v-for="px in WATERFALL" :key="px" class="fall">
          <span class="px">{{ px }}</span>
          <div class="fall-line" :dir="rtl ? 'rtl' : undefined" :style="fontStyle(px)">{{ lineText }}</div>
        </div>
      </template>
      <template v-else-if="prefs.view === 'glyphs'">
        <div class="glyph-detail" dir="ltr">
          <div v-if="picked !== null" class="glyph-big" :style="fontStyle()">{{ String.fromCodePoint(picked) }}</div>
          <div v-if="picked !== null" class="glyph-facts">
            <div class="mono">{{ codePoint(picked) }}</div>
            <div class="faint">{{ pickedName }}</div>
            <button type="button" class="copy" @click="copyGlyph"><AppIcon name="copy" :size="15" />{{ $t('formats.font.copy') }}</button>
          </div>
        </div>
        <div ref="grid" class="glyph-grid" dir="ltr" @scroll.passive="scroll = $event.target.scrollTop">
          <div class="glyph-inner" :style="{ height: rows * cell + 'px' }">
            <div v-for="g in cells" :key="g.cp" class="glyph" :class="{ on: g.cp === picked }" :style="{ left: g.left + 'px', top: g.top + 'px', width: cell + 'px', height: cell + 'px' }" @click="pick(g.cp)">
              <span class="ch" :style="fontStyle(cell * 0.5)">{{ String.fromCodePoint(g.cp) }}</span>
              <span class="cp">{{ g.cp.toString(16).toUpperCase().padStart(4, '0') }}</span>
            </div>
          </div>
        </div>
      </template>
      <template v-else>
        <div class="props">
          <div v-for="r in detailRows" :key="r.label" class="kv">
            <span>{{ r.label }}</span>
            <a v-if="r.href" :class="{ mono: r.mono }" :href="r.href" :title="r.href" @click.prevent="openUrl(r.href)" @auxclick.prevent="openUrl(r.href)" @dragstart.prevent>{{ r.value }}</a>
            <span v-else :class="{ mono: r.mono }" :title="r.value">{{ r.value }}</span>
          </div>
        </div>
        <template v-for="[label, text] in longTexts" :key="label">
          <h3>{{ label }}</h3>
          <div class="long">{{ text }}</div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.font-view { position: absolute; inset: 0; background: var(--page-bg); color: var(--page-fg); overflow: hidden; }
.view { height: 100%; box-sizing: border-box; overflow: auto; padding: calc(2.24rem * var(--g)) calc(3.36rem * var(--g)); }
.problem { padding: calc(2.24rem * var(--g)); color: var(--text-muted); }

.sample { display: flex; flex-direction: column; gap: calc(1.4rem * var(--g)); }
.sample .name { font-size: 0.92rem; opacity: 0.6; }
.sample .title, .sample .line, .sample .body { outline: none; overflow-wrap: anywhere; line-height: 1.25; }
.sample .body { line-height: 1.6; max-width: 70ch; }

.waterfall { display: flex; flex-direction: column; }
.fall { display: flex; align-items: baseline; gap: calc(1.12rem * var(--g)); padding: calc(0.42rem * var(--g)) 0; border-bottom: 1px solid color-mix(in srgb, var(--page-fg) 12%, transparent); }
.fall .px { flex: none; width: 3ch; text-align: right; font: 0.85rem var(--mono-font); opacity: 0.5; }
.fall-line { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; line-height: 1.2; }

.view.glyphs { display: flex; flex-direction: column; gap: calc(0.84rem * var(--g)); overflow: hidden; }
/* the picked glyph on the theme's own panel, so its button and facts read in any appearance */
.glyph-detail { flex: none; display: flex; align-items: center; gap: calc(1.68rem * var(--g)); min-height: round(9.79rem, 1px); padding: calc(0.56rem * var(--g)) calc(1.12rem * var(--g)); border-radius: calc(8px * var(--r)); background: var(--panel); color: var(--text); }
.glyph-big { font-size: 120px; line-height: 1; min-width: round(9.79rem, 1px); text-align: center; }
.glyph-facts { display: flex; flex-direction: column; align-items: flex-start; gap: calc(0.28rem * var(--g)); }
.glyph-facts .mono { font-family: var(--mono-font); font-size: 1.08rem; }
.copy { display: inline-flex; align-items: center; gap: calc(0.35rem * var(--g)); padding: calc(0.28rem * var(--g)) calc(0.56rem * var(--g)); border: 0; border-radius: calc(5px * var(--r)); background: none; color: var(--text-muted); font: inherit; cursor: pointer; }
.copy:hover { background: var(--hover); color: var(--text); }
.glyph-grid { flex: 1; min-height: 0; overflow-y: auto; position: relative; }
.glyph-inner { position: relative; }
.glyph { position: absolute; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; justify-content: center; border-radius: calc(6px * var(--r)); cursor: pointer; }
.glyph:hover { background: color-mix(in srgb, var(--page-fg) 8%, transparent); }
.glyph.on { background: color-mix(in srgb, var(--accent) 25%, transparent); }
.glyph .ch { line-height: 1.1; }
.glyph .cp { font: 0.63rem var(--mono-font); opacity: 0.45; }

.details { max-width: 62.94rem; }
.props { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); }
.kv { display: flex; justify-content: space-between; gap: calc(1.12rem * var(--g)); }
.kv > :last-child { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-muted); text-align: right; }
.kv a { color: var(--accent); text-decoration: none; }
.mono { font-family: var(--mono-font); }
.details h3 { margin: calc(1.68rem * var(--g)) 0 calc(0.56rem * var(--g)); }
.details .long { white-space: pre-wrap; overflow-wrap: anywhere; color: var(--text-muted); line-height: 1.5; }
</style>
