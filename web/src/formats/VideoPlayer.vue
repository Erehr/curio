<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { toast } from '../app/notify.js'
import { releaseMedia, thumbOf } from './index.js'
import { playUrl } from '../platform/fileUrls.js'
import { P, loadSiblingSubtitles, resetPlayer, showSubtitle } from './playerState.js'
import PlayerBar from './PlayerBar.vue'
import { t } from '../shared/i18n.js'

// A video: played as the file is, turned or mirrored only on screen. Settings → Preview chooses
// whether it starts by itself, fills the window and picks up where it was left.
// null while a file the player can't read gets its copy
const src = ref(null)
const props = defineProps({ item: Object, rotation: { type: Number, default: 0 }, flip: Boolean })
const el = ref(null)
const area = ref()
const box = ref({ w: 0, h: 0 })
// by the item's uid, which no item of another library shares
const POSITIONS_KEY = 'curio.video.places'
const KEEP_POSITIONS = 200
// the last few seconds count as finished
const END_MARGIN = 3
const poster = computed(() => thumbOf(props.item) || undefined)
const goingOn = ref(P.goOn)
P.goOn = false
const autoplay = computed(() => S.ui.videoAutoplay !== false || goingOn.value)
// in full screen the bar lies over the picture, shown while the pointer is near the bottom
const BAR_ZONE_PX = 110
const barShown = ref(false)
function pointer(e) {
  if (S.fullscreen) barShown.value = e.currentTarget.getBoundingClientRect().bottom - e.clientY < BAR_ZONE_PX
}
const style = computed(() => {
  const turned = props.rotation % 180 !== 0
  const parts = []
  if (props.rotation) parts.push(`rotate(${props.rotation}deg)`)
  if (props.flip) parts.push('scaleX(-1)')
  const s = parts.length ? { transform: parts.join(' ') } : {}
  // a sideways video has to fit the other way round
  if (turned && box.value.w) {
    s.maxWidth = box.value.h + 'px'
    s.maxHeight = box.value.w + 'px'
  }
  return s
})
function toggle() {
  const m = el.value
  if (!m) return
  if (m.paused) m.play().catch(() => {})
  else m.pause()
}
const positions = () => {
  try {
    return JSON.parse(localStorage.getItem(POSITIONS_KEY) || '{}')
  } catch {
    return {}
  }
}
function remember() {
  const m = el.value
  if (!m || S.ui.videoRememberPosition === false || !Number.isFinite(m.duration)) return
  const all = positions()
  if (m.currentTime > 0 && m.currentTime < m.duration - END_MARGIN) all[props.item.uid] = m.currentTime
  else delete all[props.item.uid]
  const keys = Object.keys(all)
  if (keys.length > KEEP_POSITIONS) delete all[keys[0]]
  localStorage.setItem(POSITIONS_KEY, JSON.stringify(all))
}
function metadata() {
  const m = el.value
  if (!m || S.ui.videoRememberPosition === false) return
  const t = positions()[props.item.uid]
  if (t && t < m.duration - END_MARGIN) m.currentTime = t
}
// Linux plays through the system's GStreamer, whose codecs come in optional packages
const LINUX = navigator.userAgent.includes('Linux')
function failed() {
  if (src.value) toast(LINUX ? t('formats.video.cantPlayLinux', { ext: props.item.ext }) : t('formats.video.cantPlay', { ext: props.item.ext }), 'error')
}
watch(() => props.item?.id, () => props.item && (src.value = playUrl(props.item.id)), { immediate: true })
watch(el, (m, old) => {
  P.el = m
  if (old && old !== m) releaseMedia(old)
})
watch(() => props.item?.id, (id, old) => {
  if (old != null) remember()
  goingOn.value = P.goOn
  P.goOn = false
  resetPlayer()
  if (id != null) loadSiblingSubtitles(id)
})
// a track added by Load Subtitles shows at once
watch(() => P.subtitles.length, () => setTimeout(() => showSubtitle(P.subtitle), 50))
let ro
onMounted(() => {
  P.el = el.value
  if (props.item?.id != null) loadSiblingSubtitles(props.item.id)
  ro = new ResizeObserver(() => (box.value = { w: area.value.clientWidth, h: area.value.clientHeight }))
  ro.observe(area.value)
})
onBeforeUnmount(() => {
  remember()
  ro?.disconnect()
  releaseMedia(el.value)
  P.el = null
  resetPlayer()
})
</script>

<template>
  <div class="video-player" :class="{ full: S.fullscreen }" @pointermove="pointer" @pointerleave="barShown = false">
    <div ref="area" class="video-area" @click="toggle">
      <video ref="el" :key="item.id" :src="src || undefined" :poster="poster" :autoplay="autoplay" class="video" :class="{ stretch: S.ui.videoStretch }" :style="style" @loadedmetadata="metadata" @pause="remember" @error="failed">
        <track v-for="s in P.subtitles" :key="s.url" kind="captions" srclang="en" :src="s.url" :label="s.name" />
      </video>
    </div>
    <PlayerBar :class="{ shown: barShown }" :media="el" :preview-src="src || undefined" :aspect="item.width && item.height ? item.width / item.height : 16 / 9" />
  </div>
</template>

<style scoped>
.video-player { position: absolute; inset: 0; display: flex; flex-direction: column; }
.video-area { flex: 1; min-height: 0; position: relative; display: flex; align-items: center; justify-content: center; overflow: hidden; }
.video { max-width: 100%; max-height: 100%; display: block; }
.video.stretch { width: 100%; height: 100%; object-fit: contain; }
.video-player.full .player-bar { position: absolute; left: 0; right: 0; bottom: 0; opacity: 0; pointer-events: none; transition: opacity 0.12s; border-top: 0; background: color-mix(in srgb, var(--bg) 82%, transparent); }
.video-player.full .player-bar.shown { opacity: 1; pointer-events: auto; }
</style>
