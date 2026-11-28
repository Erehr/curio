<script setup>
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue'
import Toast from 'openvue/toast'
import { useToast } from 'openvue/usetoast'
import { S } from './state.js'
import { closeToast, run } from './notify.js'
import AppIcon from '../shared/AppIcon.vue'
import { fmtTime } from '../formats/index.js'
import { englishText, t } from '../shared/i18n.js'

// The notices and live import/job progress bars at the bottom right, and the thumbnails and reads still under way after files are added; progress toasts stay until finished or cancelled.
const NOTICE_MS = 5000
const ERROR_MS = 8000
const IMPORT_ID = 'curio-import'
const JOB_PREFIX = 'curio-job-'
// the progress of imports, jobs and background work shows in the main window alone; Settings and other windows, which follow the same status, keep only their own notices
const props = defineProps({ progress: { type: Boolean, default: false } })
const toast = useToast()
const shown = new Map()
const ours = (m) => S.toasts.find((x) => 'curio-' + x.id === m.id)

const importJob = computed(() => (props.progress ? S.status.import : null))
const jobs = computed(() => (props.progress ? S.status.jobs || [] : []))
const jobOf = (m) => (typeof m.id === 'string' && m.id.startsWith(JOB_PREFIX) ? jobs.value.find((j) => JOB_PREFIX + j.id === m.id) : null)
const fraction = (done, total) => (total ? Math.min(1, done / total) : 0)

const clock = ref(Date.now())
let timer = null
// the elapsed time ticks only while an import or job is running
watch(() => !!(importJob.value || jobs.value.length), (busy) => {
  clearInterval(timer)
  timer = busy ? setInterval(() => (clock.value = Date.now()), 500) : null
  clock.value = Date.now()
}, { immediate: true })
onBeforeUnmount(() => clearInterval(timer))
const since = (started) => Math.max(0, (clock.value - started) / 1000)

// Rust titles Curio's own jobs in English (src/core/library/mod.rs, the duplicate finder's in src/core/api/operations/mod.rs)
const OWN_JOBS = ['metadata', 'duplicates']
const titleOf = (j) => {
  const own = OWN_JOBS.find((id) => englishText(`library.jobs.titles.${id}`) === j.title)
  return own ? t(`library.jobs.titles.${own}`) : j.title
}
const count = (j) => (j.total ? `${j.done.toLocaleString()}/${j.total.toLocaleString()}` : '')

watch(() => S.toasts.map((x) => x.id), (ids) => {
  for (const x of S.toasts) {
    const key = 'curio-' + x.id
    if (shown.has(key)) continue
    const m = { id: key, group: 'curio', severity: x.kind === 'error' || x.kind === 'warn' ? x.kind : 'info', styleClass: `toast ${x.kind}`, life: x.ms ?? (x.kind === 'error' ? ERROR_MS : NOTICE_MS), closable: true }
    shown.set(key, m)
    toast.add(m)
  }
  const active = new Set(ids.map((id) => 'curio-' + id))
  for (const [key, m] of shown) {
    if (key === IMPORT_ID || key.startsWith(JOB_PREFIX) || key.startsWith(BACKGROUND_PREFIX) || active.has(key)) continue
    shown.delete(key)
    toast.remove(m)
  }
}, { immediate: true })

watch(() => !!importJob.value, (active) => {
  if (active && !shown.has(IMPORT_ID)) {
    const m = { id: IMPORT_ID, group: 'curio', severity: 'info', styleClass: 'toast progress', closable: false }
    shown.set(IMPORT_ID, m)
    toast.add(m)
  } else if (!active && shown.has(IMPORT_ID)) {
    const m = shown.get(IMPORT_ID)
    shown.delete(IMPORT_ID)
    toast.remove(m)
  }
}, { immediate: true })

watch(() => jobs.value.map((j) => j.id), (ids) => {
  for (const id of ids) {
    const key = JOB_PREFIX + id
    if (shown.has(key)) continue
    const m = { id: key, group: 'curio', severity: 'info', styleClass: 'toast progress', closable: false }
    shown.set(key, m)
    toast.add(m)
  }
  const active = new Set(ids.map((id) => JOB_PREFIX + id))
  for (const [key, m] of shown) {
    if (!key.startsWith(JOB_PREFIX) || active.has(key)) continue
    shown.delete(key)
    toast.remove(m)
  }
}, { immediate: true })

// what is still made after files are added, which has no job of its own: thumbnails, and files read by plugin pages (a comic's pages)
// shown once it has gone on a while, as how much is done of all that has waited since it began; hidden by the user until it next ends
const BACKGROUND_PREFIX = 'curio-background-'
const BACKGROUND_AFTER_MS = 5000
const background = reactive({ thumbs: { done: 0, left: 0, since: 0, hidden: false }, reads: { done: 0, left: 0, since: 0, hidden: false } })
const leftOf = (kind) => (!props.progress ? 0 : kind === 'thumbs' ? (S.status.thumbs || 0) + (S.status.pageThumbs || 0) : S.status.reads || 0)
// a rescan's "Reading metadata" job already shows the reading
const readingJob = () => jobs.value.some((j) => j.title === englishText('library.jobs.titles.metadata'))
const backgroundKind = (m) => (typeof m.id === 'string' && m.id.startsWith(BACKGROUND_PREFIX) ? m.id.slice(BACKGROUND_PREFIX.length) : null)
const backgroundTitle = (kind) => (kind === 'thumbs' ? t('library.jobs.thumbnails') : t('library.jobs.titles.metadata'))
const backgroundCount = (b) => `${b.done.toLocaleString()}/${(b.done + b.left).toLocaleString()}`
function showBackground(key, on) {
  if (on && !shown.has(key)) {
    const m = { id: key, group: 'curio', severity: 'info', styleClass: 'toast progress', closable: false }
    shown.set(key, m)
    toast.add(m)
  } else if (!on && shown.has(key)) {
    toast.remove(shown.get(key))
    shown.delete(key)
  }
}
// the status is asked for every second, each answer a new object
watch(() => S.status, () => {
  const now = Date.now()
  for (const kind of Object.keys(background)) {
    const b = background[kind]
    const left = leftOf(kind)
    if (left < b.left) b.done += b.left - left
    b.left = left
    if (!left) Object.assign(b, { done: 0, since: 0, hidden: false })
    else if (!b.since) b.since = now
    // while files are still being added, their bar says enough
    showBackground(BACKGROUND_PREFIX + kind, left > 0 && !b.hidden && !importJob.value && now - b.since >= BACKGROUND_AFTER_MS && !(kind === 'reads' && readingJob()))
  }
}, { immediate: true })
function hideBackground(kind) {
  background[kind].hidden = true
  showBackground(BACKGROUND_PREFIX + kind, false)
}

function done({ message }) {
  const x = ours(message)
  if (x) closeToast(x.id)
}
function undo(message) {
  const x = ours(message)
  if (!x) return
  closeToast(x.id)
  x.undo()
}
</script>

<template>
  <Toast class="app-toasts" group="curio" position="bottom-right" :baseZIndex="2000" :pt="{ transition: { css: false } }" @close="done" @lifeEnd="done">
    <template #message="{ message }">
      <template v-if="message.id === IMPORT_ID && importJob">
        <div class="toast-fill" :class="{ endless: !importJob.total }" :style="{ width: fraction(importJob.done, importJob.total) * 100 + '%' }" />
        <span class="toast-progress-text">
          <span>{{ $t('library.jobs.adding', { n: importJob.done.toLocaleString() + (importJob.total ? '/' + importJob.total.toLocaleString() : '') }) }}</span>
          <span class="mono toast-time">({{ fmtTime(since(importJob.started)) }})</span>
        </span>
        <button class="toast-action" v-tooltip.top="$t('library.jobs.stop')" :aria-label="$t('library.jobs.stop')" @click="run('cancel_import')"><AppIcon name="x" :size="15" /></button>
      </template>
      <template v-else-if="jobOf(message)">
        <div class="toast-fill" :class="{ endless: !jobOf(message).total }" :style="{ width: fraction(jobOf(message).done, jobOf(message).total) * 100 + '%' }" />
        <span class="toast-progress-text" :data-job="jobOf(message).id">
          <span class="toast-job-title">{{ titleOf(jobOf(message)) }}</span>
          <span v-if="jobOf(message).text" class="toast-job-sub">{{ jobOf(message).text }}</span>
          <span class="mono toast-time">{{ count(jobOf(message)) }} ({{ fmtTime(since(jobOf(message).started)) }})</span>
        </span>
        <button class="toast-action" v-tooltip.top="$t('library.jobs.cancel')" :aria-label="$t('library.jobs.cancelNamed', { name: titleOf(jobOf(message)) })" @click="run('job_cancel', { id: jobOf(message).id })"><AppIcon name="x" :size="15" /></button>
      </template>
      <template v-else-if="backgroundKind(message)">
        <div class="toast-fill" :style="{ width: fraction(background[backgroundKind(message)].done, background[backgroundKind(message)].done + background[backgroundKind(message)].left) * 100 + '%' }" />
        <span class="toast-progress-text" :data-background="backgroundKind(message)">
          <span class="toast-job-title">{{ backgroundTitle(backgroundKind(message)) }}</span>
          <span class="mono toast-time">{{ backgroundCount(background[backgroundKind(message)]) }}</span>
        </span>
        <button class="toast-action" v-tooltip.top="$t('library.jobs.hide')" :aria-label="$t('library.jobs.hide')" @click="hideBackground(backgroundKind(message))"><AppIcon name="x" :size="15" /></button>
      </template>
      <template v-else>
        <AppIcon :name="message.severity === 'error' ? 'alert-circle-filled' : message.severity === 'warn' ? 'alert-triangle-filled' : 'circle-check-filled'" :size="18" class="toast-state" />
        <span class="text">{{ ours(message)?.text }}</span>
        <button v-if="ours(message)?.undo" class="toast-action" v-tooltip.top="$t('app.toasts.undo')" :aria-label="$t('app.toasts.undo')" @click="undo(message)"><AppIcon name="arrow-back-up" :size="15" /></button>
      </template>
    </template>
  </Toast>
</template>

<style>
.app-toasts.p-toast-bottom-right { right: calc(1.4rem * var(--g)); bottom: calc(1.4rem * var(--g)); width: auto; max-width: min(36.36rem, calc(100vw - 2.8rem)); }
.p-toast .p-toast-message.toast { margin: calc(0.42rem * var(--g)) 0 0; border-radius: calc(8px * var(--r)); background: var(--surface); color: var(--text); border: 1px solid var(--float-border); transition: none; }
.p-toast .p-toast-message.toast.progress { position: relative; overflow: hidden; min-width: min(24rem, calc(100vw - 2.8rem)); }
.p-toast .p-toast-message.toast.error { border-color: var(--danger); }
.p-toast .toast .p-toast-message-content { position: relative; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); min-height: round(2.52rem, 1px); padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)) calc(0.42rem * var(--g)) calc(0.84rem * var(--g)); border: 0; }
.p-toast .toast .toast-state { flex: none; color: var(--success); }
.p-toast .toast.error .toast-state { color: var(--danger); }
.p-toast .p-toast-message.toast.warn { border-color: var(--warning); }
.p-toast .toast.warn .toast-state { color: var(--warning); }
.p-toast .toast .text { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.p-toast .toast .toast-fill { position: absolute; left: 0; top: 0; bottom: 0; background: color-mix(in srgb, var(--accent) 34%, transparent); transition: width 0.2s linear; pointer-events: none; }
/* work of unknown length: a band passing along the bar */
.p-toast .toast .toast-fill.endless { width: 30% !important; animation: toast-pass 1.6s ease-in-out infinite; }
@keyframes toast-pass { from { left: -30%; } to { left: 100%; } }
.p-toast .toast .toast-progress-text { position: relative; z-index: 1; flex: 1; min-width: 0; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); white-space: nowrap; }
.p-toast .toast .toast-job-title { flex: none; max-width: 50%; overflow: hidden; text-overflow: ellipsis; }
.p-toast .toast .toast-job-sub { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; color: var(--text-muted); }
.p-toast .toast .toast-time { color: var(--text-muted); font-size: 0.92rem; margin-left: auto; }
.p-toast .toast .toast-action { position: relative; z-index: 1; flex: none; width: round(1.54rem, 1px); height: round(1.54rem, 1px); display: flex; align-items: center; justify-content: center; border: 0; padding: 0; border-radius: calc(4px * var(--r)); background: transparent; color: var(--text-muted); cursor: pointer; }
.p-toast .toast .toast-action:hover { background: var(--hover); color: var(--text); }
.p-toast .toast .p-toast-close-button { width: round(1.54rem, 1px); height: round(1.54rem, 1px); margin: 0; border-radius: calc(4px * var(--r)); color: var(--text-muted); }
.p-toast .toast .p-toast-close-button:hover { background: var(--hover); color: var(--text); }
@media (prefers-reduced-motion: reduce) { .p-toast .toast .toast-fill.endless { animation: none; left: 0; } }
</style>
