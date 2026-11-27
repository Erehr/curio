<script setup>
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import AppIcon from '../shared/AppIcon.vue'
import { fmtTime, releaseMedia } from './index.js'
import { playUrl } from '../platform/fileUrls.js'
import { loadWave } from './mediaEngine.js'
import AudioWaveform from './AudioWaveform.vue'

// Trying a clip from the grid without opening it.
// Video: starts muted (the mute button is remembered for the whole app). Sliding along the picture
// jumps through it, holding still while moving and playing on once the pointer rests.
// Audio: plays, its waveform fills the card and pressing skips there; a button pauses, and that is
// remembered too. Both use the player's volume.
// compact: a list row's small picture, with the clip or waveform and its progress only, its buttons and text left out
const props = defineProps({ item: Object, compact: Boolean })
const root = ref()
const el = ref()
const now = ref(0)
const total = ref(0)
const ready = ref(false)
const hover = ref(null)
const showTime = ref(false)
const wave = shallowRef(null)
const isAudio = computed(() => props.item.kind === 'audio')
const muted = computed(() => S.ui.hoverMuted !== false)
const audioPlays = computed(() => S.ui.hoverAudioAutoplay !== false)
const fraction = computed(() => (total.value ? Math.min(1, now.value / total.value) : 0))
// how long the pointer rests before anything starts
const VIDEO_START_MS = 300
const AUDIO_START_MS = 250
// how often moves count, how soon the clip plays on after the last one, how long the time stays visible
const MOVE_EVERY_MS = 40
const RESUME_AFTER_MS = 300
const TIME_SHOWN_MS = 900
// a song starts over shortly after it ends
const RESTART_AFTER_MS = 400
const src = ref(null)
// compact video lies over the thumbnail beside it, which is fitted inside a box of another shape, so the bar runs along the picture's edge
const over = ref(null)
let startTimer = null
let resumeTimer = null
let timeTimer = null
let lastMove = 0
let frame = 0

function sync() {
  const m = el.value
  if (!m) return
  now.value = m.currentTime
  if (Number.isFinite(m.duration)) total.value = m.duration
}
// the bar follows the clip every frame, not only when the browser reports the time
function tick() {
  sync()
  frame = requestAnimationFrame(tick)
}
function setUp() {
  const m = el.value
  if (m) m.volume = S.ui.playerVolume ?? 1
  sync()
}
function pointerFraction(e) {
  const r = root.value.getBoundingClientRect()
  return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width))
}
// a seek is only sent when the last one has landed, or a long clip falls behind the pointer
let seeking = false
let wanted = null
function seek(f) {
  const m = el.value
  if (!m || !total.value) return
  if (seeking) {
    wanted = f
    return
  }
  seeking = true
  m.currentTime = Math.round(f * total.value * 100) / 100
}
function seeked() {
  seeking = false
  ready.value = true
  if (wanted != null) {
    const f = wanted
    wanted = null
    seek(f)
  }
}
function move(e) {
  const f = pointerFraction(e)
  hover.value = f
  if (isAudio.value) return
  const t = performance.now()
  if (t - lastMove < MOVE_EVERY_MS) return
  lastMove = t
  seek(f)
  const m = el.value
  if (!m) return
  m.pause()
  showTime.value = true
  clearTimeout(timeTimer)
  timeTimer = setTimeout(() => (showTime.value = false), TIME_SHOWN_MS)
  clearTimeout(resumeTimer)
  resumeTimer = setTimeout(() => el.value?.play().catch(() => {}), RESUME_AFTER_MS)
}
function press(e) {
  if (isAudio.value && e.button === 0 && !e.target.closest('button')) seek(pointerFraction(e))
}
function ended() {
  setTimeout(() => {
    const m = el.value
    if (!m) return
    m.currentTime = 0
    m.play().catch(() => {})
  }, RESTART_AFTER_MS)
}
function toggleMute() {
  S.ui.hoverMuted = !muted.value
  saveUi()
}
function toggleAudio() {
  const m = el.value
  S.ui.hoverAudioAutoplay = !audioPlays.value
  if (m) {
    if (S.ui.hoverAudioAutoplay) m.play().catch(() => {})
    else m.pause()
  }
  saveUi()
}
onMounted(() => {
  const pic = props.compact && !isAudio.value ? root.value.parentElement?.querySelector(':scope > img') : null
  if (pic) over.value = { inset: 'auto', left: pic.offsetLeft + 'px', top: pic.offsetTop + 'px', width: pic.offsetWidth + 'px', height: pic.offsetHeight + 'px' }
  startTimer = setTimeout(async () => {
    src.value = playUrl(props.item.id)
    if (isAudio.value) {
      try {
        wave.value = await loadWave(props.item.id)
      } catch {
        wave.value = new Uint8Array(0)
      }
    }
  }, isAudio.value ? AUDIO_START_MS : VIDEO_START_MS)
  frame = requestAnimationFrame(tick)
})
onBeforeUnmount(() => {
  clearTimeout(startTimer)
  clearTimeout(resumeTimer)
  clearTimeout(timeTimer)
  cancelAnimationFrame(frame)
  releaseMedia(el.value)
})
</script>

<template>
  <div ref="root" class="hover-player" :class="{ audio: isAudio, compact }" :style="over" @pointermove="move" @pointerleave="hover = null" @pointerdown="press">
    <template v-if="src">
      <audio v-if="isAudio" ref="el" :src="src" :autoplay="audioPlays" @loadedmetadata="setUp" @seeked="seeked" @ended="ended" />
      <video v-else ref="el" :src="src" :muted="muted" autoplay loop playsinline class="clip" :class="{ ready }" @loadedmetadata="setUp" @playing="ready = true" @seeked="seeked" />
    </template>

    <template v-if="isAudio">
      <div class="dim" />
      <AudioWaveform v-if="wave?.length" :peaks="wave" :played="fraction" class="audio-wave" />
      <div class="now" :style="{ left: fraction * 100 + '%' }" />
      <div v-if="hover != null" class="line" :style="{ left: hover * 100 + '%' }" />
      <div v-if="!compact" class="audio-foot"><span class="ellipsis">{{ item.title }}</span><span class="mono">{{ fmtTime(hover != null ? hover * total : now) }}</span></div>
      <button v-if="!compact" class="auto" v-tooltip.top="audioPlays ? $t('formats.player.pause') : $t('formats.player.playOnHover')" @pointerdown.stop @click.stop="toggleAudio" @dblclick.stop>
        <AppIcon :name="audioPlays ? 'player-pause-filled' : 'player-play-filled'" :size="14" />
      </button>
    </template>
    <template v-else>
      <div class="bar"><div class="bar-done" :style="{ width: fraction * 100 + '%' }" /></div>
      <span v-if="!compact" class="stamp mono" :class="{ show: showTime }">{{ fmtTime(now) }}</span>
      <button v-if="!compact" class="mute" v-tooltip.top="muted ? $t('formats.player.unmute') : $t('formats.player.mute')" @click.stop="toggleMute" @pointerdown.stop @dblclick.stop>
        <AppIcon :name="muted ? 'volume-off' : 'volume'" :size="15" />
      </button>
    </template>
  </div>
</template>

<style scoped>
.hover-player { position: absolute; inset: 0; z-index: 2; overflow: hidden; }
.clip { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block; opacity: 0; }
.clip.ready { opacity: 1; }
.compact { border-radius: calc(3px * var(--r)); }
.compact .clip { object-fit: contain; }
.compact .audio-wave { top: 15%; height: 70%; }
.bar { position: absolute; left: 0; right: 0; bottom: 0; height: 0.21rem; background: rgba(255, 255, 255, 0.25); }
.bar-done { height: 100%; background: var(--accent); }
.stamp { position: absolute; left: 0.42rem; bottom: 0.49rem; padding: 1px calc(0.35rem * var(--g)); border-radius: calc(4px * var(--r)); background: rgba(0, 0, 0, 0.6); color: #fff; font-size: 0.85rem; opacity: 0; transition: opacity 0.12s; }
.stamp.show { opacity: 1; }
.auto { position: absolute; right: 0.42rem; top: 0.42rem; width: round(1.68rem, 1px); height: round(1.68rem, 1px); display: flex; align-items: center; justify-content: center; border: 0; border-radius: calc(5px * var(--r)); background: rgba(0, 0, 0, 0.6); color: #fff; cursor: pointer; }
.mute { position: absolute; right: 0.42rem; bottom: 0.49rem; width: round(1.68rem, 1px); height: round(1.68rem, 1px); display: flex; align-items: center; justify-content: center; border: 0; border-radius: calc(5px * var(--r)); background: rgba(0, 0, 0, 0.6); color: #fff; cursor: pointer; }
.mute:hover { background: rgba(0, 0, 0, 0.85); }
.dim { position: absolute; inset: 0; background: rgba(15, 16, 18, 0.72); }
.audio-wave { position: absolute; left: 0; top: 10%; width: 100%; height: 68%; }
.now { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--accent); pointer-events: none; }
.line { position: absolute; top: 0; bottom: 0; width: 0.14rem; margin-left: -1px; background: #fff; pointer-events: none; }
.audio-foot { position: absolute; left: 0.56rem; right: 0.56rem; bottom: 0.42rem; display: flex; gap: calc(0.56rem * var(--g)); align-items: center; justify-content: space-between; color: #fff; font-size: 0.92rem; pointer-events: none; }
.audio-foot .mono { flex: none; }
</style>
