<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Slider from 'openvue/slider'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { stepViewer, viewerIdAt, viewerPosition } from '../viewer/session.js'
import AppIcon from '../shared/AppIcon.vue'
import AppMenu from '../shared/AppMenu.vue'
import { fmtTime, kindOf, releaseMedia } from './index.js'
import AudioWaveform from './AudioWaveform.vue'
import { AUDIO_SPEEDS, VIDEO_SPEEDS, P, showSubtitle } from './playerState.js'
import { t } from '../shared/i18n.js'

// The controls under a video or a song: the same bar for both, with a waveform for the timeline of
// songs. Volume, mute, speed and what happens at the end are remembered across items.
const props = defineProps({ media: Object, wave: Object, ready: Boolean, previewSrc: String, aspect: { type: Number, default: 16 / 9 } })
const playing = ref(false)
const now = ref(0)
const total = ref(0)
const hoverAt = ref(null)
const line = ref()
// speed and captions are menus of the app's own kind
const speedMenu = ref()
const captionsMenu = ref()
const speedItems = () => SPEEDS.value.map((s) => ({ label: `${s}x`, current: s === speed.value, command: () => setUi('playerSpeed', s) }))
const captionItems = () => [{ label: t('formats.player.captionsOff'), current: !P.subtitle, command: () => showSubtitle(null) }, ...P.subtitles.map((c) => ({ label: c.name, current: P.subtitle === c.name, command: () => showSubtitle(c.name) }))]
const SPEEDS = computed(() => (props.media?.tagName === 'AUDIO' ? AUDIO_SPEEDS : VIDEO_SPEEDS))
// a song keeps within its own, shorter list
const speed = computed(() => {
  const s = S.ui.playerSpeed || 1
  return SPEEDS.value.includes(s) ? s : 1
})
const volume = computed(() => S.ui.playerVolume ?? 1)
const muted = computed(() => !!S.ui.playerMuted)
// at the end: play it again, go on to the next item, or stop
const END_MODES = { repeat: 'repeat-once', next: 'player-track-next', stop: 'repeat-off' }
const NEXT_MODE = { repeat: 'next', next: 'stop', stop: 'repeat' }
const endMode = computed(() => (END_MODES[S.ui.playerEnd] ? S.ui.playerEnd : 'repeat'))
const loop = computed(() => endMode.value === 'repeat')
const viewPos = computed(() => {
  void S.resultsVer
  return viewerPosition()
})
const canPrev = computed(() => S.viewer && viewPos.value.pos > 0)
const canNext = computed(() => S.viewer && viewPos.value.pos >= 0 && viewPos.value.pos < viewPos.value.length - 1)

function applySettings() {
  const m = props.media
  if (!m) return
  m.volume = volume.value
  m.muted = muted.value
  m.playbackRate = speed.value
  m.loop = loop.value
}
function sync() {
  const m = props.media
  if (!m) return
  playing.value = !m.paused
  now.value = m.currentTime
  total.value = Number.isFinite(m.duration) ? m.duration : 0
}
const EVENTS = ['play', 'pause', 'timeupdate', 'durationchange', 'loadedmetadata', 'seeked']
function ended() {
  if (endMode.value !== 'next' || !canNext.value) return
  P.goOn = kindOf(S.items.get(viewerIdAt(viewPos.value.pos + 1))) === 'video'
  stepViewer(1)
}
function attach(m, old) {
  for (const e of EVENTS) {
    old?.removeEventListener(e, sync)
    m?.addEventListener(e, sync)
  }
  old?.removeEventListener('ended', ended)
  m?.addEventListener('ended', ended)
  applySettings()
  sync()
}
watch(() => props.media, attach, { immediate: true })
onBeforeUnmount(() => attach(null, props.media))
watch(() => [volume.value, muted.value, speed.value, loop.value], applySettings)

// smooth playhead between the media element's coarse time events, only while playing
let raf = 0
function tick() {
  raf = 0
  const m = props.media
  if (!m || m.paused) return
  now.value = m.currentTime
  // the handles on the timeline: play only this part, over and over
  if (m.duration && (P.loopIn > 0 || P.loopOut < 1)) {
    const end = P.loopOut * m.duration
    const start = P.loopIn * m.duration
    if (m.currentTime >= end || m.currentTime < start) m.currentTime = start
  }
  raf = requestAnimationFrame(tick)
}
watch(playing, (on) => {
  if (on && !raf) raf = requestAnimationFrame(tick)
}, { immediate: true })
onBeforeUnmount(() => cancelAnimationFrame(raf))

function toggle() {
  const m = props.media
  if (!m) return
  if (m.paused) m.play().catch(() => {})
  else m.pause()
}
function setUi(k, v) {
  S.ui[k] = v
  saveUi()
}
function setVolume(v) {
  S.ui.playerVolume = v
  S.ui.playerMuted = v === 0
  saveUi()
}
const fraction = computed(() => (total.value ? Math.min(1, now.value / total.value) : 0))
function atPointer(e) {
  const r = line.value.getBoundingClientRect()
  return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width))
}
function seekTo(f) {
  const m = props.media
  if (m && total.value) {
    m.currentTime = f * total.value
    now.value = m.currentTime
  }
}
// the frame under the pointer, from a second copy of the video that only ever seeks; it exists only while the pointer is over the bar
const peek = ref()
let peekBusy = false
let peekWanted = null
watch(peek, (v, old) => {
  if (old && old !== v) releaseMedia(old)
  peekBusy = false
  peekWanted = v ? hoverAt.value : null
})
onBeforeUnmount(() => releaseMedia(peek.value))
function peekAt(f) {
  const v = peek.value
  if (!v || !total.value) return
  if (peekBusy || v.readyState < 1) {
    peekWanted = f
    return
  }
  peekBusy = true
  v.currentTime = f * total.value
}
function peekDone() {
  peekBusy = false
  if (peekWanted != null) {
    const f = peekWanted
    peekWanted = null
    peekAt(f)
  }
}
function lineMove(e) {
  hoverAt.value = atPointer(e)
  peekAt(hoverAt.value)
}
const LOOP_MIN_GAP = 0.01
function dragTrim(e, which) {
  e.stopPropagation()
  const move = (ev) => {
    const f = atPointer(ev)
    if (which === 'in') P.loopIn = Math.min(f, P.loopOut - LOOP_MIN_GAP)
    else P.loopOut = Math.max(f, P.loopIn + LOOP_MIN_GAP)
    seekTo(which === 'in' ? P.loopIn : P.loopOut)
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
function lineDown(e) {
  seekTo(atPointer(e))
  const move = (ev) => seekTo(atPointer(ev))
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
const volIcon = computed(() => (muted.value || volume.value === 0 ? 'volume-off' : volume.value < 0.5 ? 'volume-2' : 'volume'))
</script>

<template>
  <div class="player-bar" @pointerdown.stop @dblclick.stop @click.stop @wheel.stop>
    <button class="pb" v-tooltip.top="playing ? $t('formats.player.pause') : $t('formats.player.play')" @click="toggle"><AppIcon :name="playing ? 'player-pause' : 'player-play'" :size="19" /></button>
    <button class="pb" :disabled="!canPrev" v-tooltip.top="$t('formats.player.previous')" @click="stepViewer(-1)"><AppIcon name="player-skip-back" :size="18" /></button>
    <button class="pb" :disabled="!canNext" v-tooltip.top="$t('formats.player.next')" @click="stepViewer(1)"><AppIcon name="player-skip-forward" :size="18" /></button>
    <div class="vol">
      <button class="pb" v-tooltip.top="muted ? $t('formats.player.unmute') : $t('formats.player.mute')" @click="setUi('playerMuted', !muted)"><AppIcon :name="volIcon" :size="19" /></button>
      <Slider class="vol-range" :min="0" :max="1" :step="0.01" :modelValue="muted ? 0 : volume" @update:modelValue="setVolume" />
    </div>
    <span class="time mono"><b>{{ fmtTime(now) }}</b><i>/</i><span>{{ fmtTime(total) }}</span></span>
    <div ref="line" class="line" :class="{ waved: wave }" @pointerdown="lineDown" @pointermove="lineMove" @pointerleave="hoverAt = null">
      <AudioWaveform v-if="wave" :peaks="wave" :played="fraction" class="line-wave" />
      <template v-else>
        <div class="track" />
        <div class="range" :style="{ left: P.loopIn * 100 + '%', right: (1 - P.loopOut) * 100 + '%' }" />
        <div class="done" :style="{ left: P.loopIn * 100 + '%', width: Math.max(0, fraction - P.loopIn) * 100 + '%' }" />
        <span class="trim" :style="{ left: P.loopIn * 100 + '%' }" v-tooltip.top="$t('formats.player.loopStart')" @pointerdown="dragTrim($event, 'in')" />
        <span class="trim" :style="{ left: P.loopOut * 100 + '%' }" v-tooltip.top="$t('formats.player.loopEnd')" @pointerdown="dragTrim($event, 'out')" />
      </template>
      <div class="head" :style="{ left: fraction * 100 + '%' }" />
      <template v-if="hoverAt != null">
        <div class="hover" :style="{ left: hoverAt * 100 + '%' }" />
        <div class="tip mono" :style="{ left: hoverAt * 100 + '%' }">{{ fmtTime(hoverAt * total) }}</div>
      </template>
      <div v-if="previewSrc && hoverAt != null" class="peek" :style="{ left: hoverAt * 100 + '%', aspectRatio: aspect }">
        <video ref="peek" :src="previewSrc" muted preload="metadata" @loadedmetadata="peekDone" @seeked="peekDone" />
      </div>
    </div>
    <div class="speed">
      <button class="pb text" v-tooltip.top="$t('formats.player.speed')" @click="speedMenu.toggle($event)">{{ speed }}x</button>
      <AppMenu ref="speedMenu" :model="speedItems" />
    </div>
    <div v-if="P.subtitles.length" class="speed">
      <button class="pb" :class="{ on: P.subtitle }" v-tooltip.top="$t('formats.player.captions')" @click="captionsMenu.toggle($event)"><AppIcon name="badge-cc" :size="19" /></button>
      <AppMenu ref="captionsMenu" :model="captionItems" />
    </div>
    <button class="pb" :class="{ on: endMode !== 'stop' }" v-tooltip.top="$t(`formats.player.end.${endMode}`)" @click="setUi('playerEnd', NEXT_MODE[endMode])"><AppIcon :name="END_MODES[endMode]" :size="18" /></button>
    <button class="pb" v-tooltip.top="$t('formats.player.fullScreen')" @click="S.fullscreen = !S.fullscreen"><AppIcon :name="S.fullscreen ? 'arrows-minimize' : 'maximize'" :size="18" /></button>
  </div>
</template>

<style scoped>
.player-bar { flex: none; display: flex; align-items: center; gap: calc(0.42rem * var(--g)); height: round(3.92rem, 1px); padding: 0 calc(0.98rem * var(--g)); background: var(--bg); border-top: 1px solid var(--border); color: var(--text); cursor: default; }
.pb { display: inline-flex; align-items: center; justify-content: center; min-width: round(2.1rem, 1px); height: round(2.1rem, 1px); padding: 0 calc(0.28rem * var(--g)); border: 0; border-radius: calc(6px * var(--r)); background: transparent; color: var(--text-muted); font: inherit; cursor: pointer; flex: none; }
.pb:hover:not(:disabled) { background: var(--hover); color: var(--text); }
.pb:disabled { opacity: 0.35; cursor: default; }
.pb.on { color: var(--accent); }
.pb.text { font-weight: 600; color: var(--text); font-size: 1rem; }
.vol { display: flex; align-items: center; flex: none; }
.vol-range { width: 0; opacity: 0; transition: width 0.12s, opacity 0.12s, margin 0.12s; margin: 0; }
.vol:hover .vol-range, .vol:focus-within .vol-range { width: round(5.03rem, 1px); opacity: 1; margin: 0 calc(0.84rem * var(--g)) 0 calc(0.42rem * var(--g)); }
.time { display: inline-flex; gap: calc(0.42rem * var(--g)); margin: 0 calc(0.56rem * var(--g)) 0 0.14rem; font-size: 1rem; flex: none; }
.time b { font-weight: 400; }
.time i { font-style: normal; color: var(--text-faint); }
.time span { color: var(--text-faint); }
.line { position: relative; flex: 1; min-width: round(4.2rem, 1px); height: 100%; cursor: pointer; display: flex; align-items: center; touch-action: none; }
.track, .done { position: absolute; left: 0; height: 0.28rem; border-radius: calc(2px * var(--r)); top: 50%; margin-top: -0.14rem; }
.track { right: 0; background: color-mix(in srgb, var(--text) 28%, transparent); }
.range { position: absolute; top: 50%; height: 0.28rem; margin-top: -0.14rem; background: color-mix(in srgb, var(--text) 55%, transparent); }
.done { background: var(--accent); }
.trim { position: absolute; top: 1.26rem; bottom: 1.26rem; width: 0.28rem; margin-left: -0.14rem; border-radius: calc(2px * var(--r)); background: var(--text); cursor: ew-resize; z-index: 2; }
.peek { position: absolute; bottom: calc(100% + 1.82rem); width: round(13.99rem, 1px); transform: translateX(-50%); border-radius: calc(8px * var(--r)); overflow: hidden; background: #000; border: 1px solid var(--border); pointer-events: none; z-index: 3; }
.peek video { width: 100%; height: 100%; display: block; object-fit: contain; }
.line-wave { position: absolute; left: 0; top: 0.7rem; width: 100%; height: calc(100% - 1.4rem); }
.head { position: absolute; top: 0.7rem; bottom: 0.7rem; width: 0.14rem; margin-left: -1px; background: var(--accent); }
.line:not(.waved) .head { width: 0.28rem; margin-left: -0.14rem; top: 1.26rem; bottom: 1.26rem; border-radius: calc(2px * var(--r)); background: var(--text); }
.hover { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--text); pointer-events: none; }
.tip { position: absolute; bottom: calc(100% - 0.28rem); transform: translateX(-50%); padding: calc(0.21rem * var(--g)) calc(0.49rem * var(--g)); border-radius: calc(5px * var(--r)); background: var(--surface-2); border: 1px solid var(--border); font-size: 0.92rem; white-space: nowrap; pointer-events: none; z-index: 3; }
.speed { position: relative; flex: none; }
</style>
