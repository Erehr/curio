<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { call, errorText } from '../platform/ipc.js'
import { toast } from '../app/notify.js'
import { textUrl } from '../platform/fileUrls.js'
import { MARKDOWN, renderMarkdown } from './markdown.js'
import { setTextPane, textView } from '../viewer/viewerControls.js'

// Text files as a page down the middle of the viewer: Markdown rendered, as written or both side by side; JSON formatted; everything else as it is.
const props = defineProps({ item: Object, compact: Boolean })
const text = ref('')
const failed = ref(null)
const SMALL_PREVIEW = 3000
const ext = computed(() => props.item?.ext || '')
const tv = textView
const mode = computed(() => (MARKDOWN.includes(ext.value) && !props.compact ? tv.value.mode : 'source'))
const sourceEl = ref()
const renderedEl = ref()

let asked = 0
watch(() => [props.item?.id, props.item?.thumb_ver], async ([id], old) => {
  const mine = ++asked
  failed.value = null
  try {
    const t = await (await fetch(textUrl(props.item.id))).text()
    if (mine !== asked) return
    text.value = t
    // another file starts at its top; the same file read again stays where it was
    if (id !== old?.[0]) nextTick(() => [sourceEl.value, renderedEl.value].forEach((el) => el && (el.scrollTop = 0)))
  } catch (e) {
    if (mine === asked) failed.value = errorText(e)
  }
}, { immediate: true })

const pretty = computed(() => {
  if (ext.value !== 'json') return text.value
  try {
    return JSON.stringify(JSON.parse(text.value), null, 2)
  } catch {
    return text.value
  }
})
const rendered = computed(() => (mode.value === 'source' ? '' : renderMarkdown(text.value)))

const showLines = computed(() => !props.compact && tv.value.lines)
// a last line break ends the last line rather than starting another
const lineList = computed(() => (showLines.value ? pretty.value.replace(/\r?\n$/, '').split(/\r?\n/) : []))
const esc = (s) => s.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c])
const lineHtml = computed(() => lineList.value.map((l) => `<div>${esc(l)}</div>`).join(''))

// the keys that scroll text scroll the pane being read
watch([sourceEl, renderedEl], ([s, r]) => !props.compact && setTextPane(r || s), { immediate: true })
onBeforeUnmount(() => !props.compact && setTextPane(null))

// a larger or smaller size, another view or other wrapping keeps the same part of the text in sight
const share = (el) => (el && el.scrollHeight > el.clientHeight ? el.scrollTop / (el.scrollHeight - el.clientHeight) : 0)
const toShare = (el, s) => el && (el.scrollTop = s * (el.scrollHeight - el.clientHeight))
watch(() => [tv.value.zoom, tv.value.mode, tv.value.wrap, tv.value.lines], () => {
  const s = share(renderedEl.value || sourceEl.value)
  nextTick(() => {
    toShare(sourceEl.value, s)
    toShare(renderedEl.value, s)
  })
}, { flush: 'pre' })

// side by side, the pane being scrolled leads and the other follows, until it has been still for a moment
const LEAD_MS = 120
let leader = null
let leadTimer = null
function follow(from, to) {
  if (mode.value !== 'split' || !from || !to || (leader && leader !== from)) return
  leader = from
  clearTimeout(leadTimer)
  leadTimer = setTimeout(() => (leader = null), LEAD_MS)
  toShare(to, share(from))
}
onBeforeUnmount(() => clearTimeout(leadTimer))

function clickRendered(e) {
  const a = e.target.closest?.('a')
  if (!a) return
  e.preventDefault()
  const href = a.getAttribute('href') || ''
  if (href.startsWith('#')) renderedEl.value?.querySelector(`[data-anchor="${CSS.escape(decodeURIComponent(href.slice(1)))}"]`)?.scrollIntoView({ block: 'start' })
  else if (/^https?:\/\//i.test(href)) call('open_url', { url: href }).catch((err) => toast(errorText(err), 'error'))
}
</script>

<template>
  <div class="text-view" :class="{ compact, 'fills-stage': !compact }" :style="{ '--tz': tv.zoom }">
    <div v-if="failed" class="faint failed">{{ failed }}</div>
    <pre v-else-if="compact" class="small">{{ pretty.slice(0, SMALL_PREVIEW) }}</pre>
    <template v-else>
      <div v-if="mode !== 'rendered'" ref="sourceEl" class="pane" @scroll="follow(sourceEl, renderedEl)">
        <div v-if="showLines" class="page mono lines" :class="{ wrap: tv.wrap }" :style="{ '--digits': String(lineList.length).length }" v-html="lineHtml" />
        <pre v-else class="page mono" :class="{ wrap: tv.wrap }">{{ pretty }}</pre>
      </div>
      <div v-if="mode !== 'source'" ref="renderedEl" class="pane" @scroll="follow(renderedEl, sourceEl)">
        <article class="page md" v-html="rendered" @click="clickRendered" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.text-view { position: absolute; inset: 0; display: flex; background: var(--bg); user-select: text; cursor: auto; }
.pane { flex: 1; min-width: 0; overflow: auto; }
.pane + .pane { border-left: 1px solid var(--border); }
.page { box-sizing: border-box; max-width: calc(60.14rem * var(--tz)); margin: 0 auto; padding: calc(2.24rem * var(--g)) calc(2.8rem * var(--g)) calc(5.03rem * var(--g)); color: var(--text); font-size: calc(0.98rem * var(--tz)); line-height: 1.6; }
.mono { font-family: var(--mono-font); font-size: calc(0.91rem * var(--tz)); line-height: 1.55; tab-size: 4; white-space: pre; }
.mono.wrap { white-space: pre-wrap; overflow-wrap: anywhere; }
/* unwrapped, long lines run on to the right and the pane scrolls sideways */
.mono:not(.wrap) { max-width: none; width: max-content; min-width: min(100%, calc(60.14rem * var(--tz))); }
.lines { counter-reset: line; }
.lines :deep(div) { position: relative; min-height: 1.55em; padding-left: calc(var(--digits) * 1ch + 2ch); }
.lines :deep(div)::before { counter-increment: line; content: counter(line); position: absolute; left: 0; width: calc(var(--digits) * 1ch); text-align: right; color: var(--text-faint); }
.failed { margin: auto; }

.md { font-family: var(--ui-font); overflow-wrap: break-word; }
.md > :deep(:first-child) { margin-top: 0; }
.md :deep(:is(p, ul, ol, dl, blockquote, pre, table, details)) { margin: 0 0 1em; }
.md :deep(:is(h1, h2, h3, h4, h5, h6)) { margin: 1.5em 0 0.6em; font-weight: 600; line-height: 1.3; }
.md :deep(h1) { font-size: 1.9em; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); }
.md :deep(h2) { font-size: 1.45em; padding-bottom: 0.2em; border-bottom: 1px solid var(--border); }
.md :deep(h3) { font-size: 1.2em; }
.md :deep(h4) { font-size: 1.05em; }
.md :deep(:is(h5, h6)) { font-size: 1em; color: var(--text-muted); }
.md :deep(:is(ul, ol)) { padding-left: 1.6em; }
.md :deep(li + li) { margin-top: 0.25em; }
.md :deep(li > :is(ul, ol)) { margin: 0.25em 0 0; }
.md :deep(li > input[type='checkbox']) { margin: 0 0.45em 0 0; vertical-align: -0.1em; }
.md :deep(a) { color: var(--accent); text-decoration: none; }
.md :deep(a[href]:hover) { text-decoration: underline; }
.md :deep(a:not([href])) { color: inherit; text-decoration: underline dotted; }
.md :deep(:is(code, kbd, samp)) { font-family: var(--mono-font); font-size: 0.88em; }
.md :deep(code) { padding: 0.1em 0.35em; border-radius: calc(4px * var(--r)); background: var(--surface-2); }
.md :deep(pre) { padding: calc(0.84rem * var(--g)) calc(0.98rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--surface-2); overflow: auto; line-height: 1.45; }
.md :deep(pre code) { padding: 0; background: none; }
.md :deep(kbd) { padding: 0 0.35em; border: 1px solid var(--border); border-radius: calc(4px * var(--r)); }
.md :deep(blockquote) { margin-left: 0; margin-right: 0; padding-left: 1em; border-left: 3px solid var(--border); color: var(--text-muted); }
.md :deep(table) { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; }
.md :deep(:is(th, td)) { padding: 0.35em 0.75em; border: 1px solid var(--border); }
.md :deep(th) { background: var(--surface-2); font-weight: 600; }
.md :deep(hr) { margin: 1.75em 0; border: 0; border-top: 1px solid var(--border); }
.md :deep(img) { max-width: 100%; }
.md :deep(mark) { background: color-mix(in srgb, var(--accent) 30%, transparent); color: inherit; }
.md :deep(.md-image) { color: var(--text-faint); font-style: italic; }

.text-view.compact { overflow: hidden; padding: calc(0.7rem * var(--g)) calc(0.84rem * var(--g)); background: var(--surface); border-radius: calc(8px * var(--r)); }
.small { flex: 1; min-width: 0; margin: 0; font-family: var(--mono-font); font-size: 0.68rem; white-space: pre-wrap; word-break: break-word; color: var(--text-muted); }
</style>
