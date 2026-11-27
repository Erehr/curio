<script setup>
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { playUrl, thumbUrl } from '../platform/fileUrls.js'
import AppIcon from '../shared/AppIcon.vue'
import { releaseMedia } from './index.js'
import { loadWave } from './mediaEngine.js'
import PlayerBar from './PlayerBar.vue'

// A song: its cover stays on screen while it plays, with the waveform as the timeline.
const props = defineProps({ item: Object })
const el = ref(null)
const wave = shallowRef(null)
const hasCover = computed(() => props.item?.thumb_state === 1)
const cover = computed(() => (hasCover.value ? thumbUrl(props.item.uid, props.item.thumb_ver) : null))
const smallCover = computed(() => (hasCover.value ? thumbUrl(props.item.uid, props.item.thumb_ver) : null))
watch(() => props.item?.id, async (id) => {
  wave.value = null
  if (id == null) return
  try {
    const b = await loadWave(id)
    if (id === props.item?.id) wave.value = b
  } catch {
    /* no waveform: a plain line */
  }
}, { immediate: true })
watch(el, (m, old) => old && old !== m && releaseMedia(old))
onBeforeUnmount(() => releaseMedia(el.value))
function toggle() {
  const m = el.value
  if (!m) return
  if (m.paused) m.play().catch(() => {})
  else m.pause()
}
</script>

<template>
  <div class="audio-player">
    <div class="audio-area" @click="toggle">
      <div class="cover" :style="smallCover ? { backgroundImage: `url(${smallCover})` } : {}">
        <img v-if="cover" :src="cover" draggable="false" />
        <AppIcon v-else name="music" :size="96" class="faint" />
      </div>
      <div class="title ellipsis">{{ item.title }}</div>
      <audio ref="el" :key="item.id" :src="playUrl(item.id)" autoplay />
    </div>
    <PlayerBar :media="el" :wave="wave" />
  </div>
</template>

<style scoped>
.audio-player { position: absolute; inset: 0; display: flex; flex-direction: column; }
.audio-area { flex: 1; min-height: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: calc(1.12rem * var(--g)); padding: calc(1.12rem * var(--g)); }
.cover { flex: 0 1 auto; min-height: 0; height: min(72%, 39.16rem); aspect-ratio: 1; display: flex; align-items: center; justify-content: center; border-radius: calc(10px * var(--r)); overflow: hidden; background: var(--grid-item) center / cover no-repeat; }
.cover img { width: 100%; height: 100%; object-fit: cover; display: block; }
.title { max-width: 80%; font-size: 1.15rem; color: var(--text-muted); flex: none; }
</style>
