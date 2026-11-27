<script setup>
import { computed, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { ensureItems } from '../library/results.js'
import AppIcon from '../shared/AppIcon.vue'
import { KIND_ICON, isAnimated, kindOf, thumbOf } from './index.js'
import { originalUrl } from '../platform/fileUrls.js'
import ImageView from './ImageView.vue'
import VideoPlayer from './VideoPlayer.vue'
import AudioPlayer from './AudioPlayer.vue'
import TextView from './TextView.vue'
import LinkView from './LinkView.vue'
import PluginFrame from '../plugins/PluginFrame.vue'
import FontView from './FontView.vue'

// One item, shown by the viewer for its type. `compact` is the small version for the Inspector.
const props = defineProps({ id: Number, zoom: { type: Number, default: 1 }, pan: { type: Object, default: () => ({ x: 0, y: 0 }) }, compact: Boolean, rotation: { type: Number, default: 0 }, flip: Boolean })
const item = computed(() => {
  void S.itemsVer
  ensureItems([props.id])
  return S.items.get(props.id)
})
const kind = computed(() => kindOf(item.value))
// the item as last known: a changed file drops it from the cache until it is read again, which mustn't reload a viewer plugin's page
const held = ref(null)
watch(item, (v) => v && (held.value = v), { immediate: true })
const known = computed(() => item.value || (held.value?.id === props.id ? held.value : null))
// a Format plugin's viewer takes its extensions over from Curio's own; the Inspector's small preview keeps the thumbnail
const viewer = computed(() => (!props.compact && known.value ? S.contrib.viewers?.find((v) => v.exts.includes(String(known.value.ext).toLowerCase())) : null))
const shown = computed(() => known.value && { id: known.value.uid, ext: known.value.ext, kind: known.value.kind, name: known.value.name })
const thumb = computed(() => thumbOf(item.value))
const failed = ref(false)
watch(() => props.id, () => (failed.value = false))
</script>

<template>
  <div class="preview" :class="{ compact }">
    <!-- a viewer plugin's page stays loaded while its file is replaced or briefly gone, and says itself when it can't read it -->
    <PluginFrame v-if="viewer?.page" class="viewer-page" :plugin="viewer.plugin" :page="viewer.page" role="viewer" :run="{ item: shown.id }" :item="shown" />
    <template v-else-if="item">
      <template v-if="item.status !== 'Ok' && thumb">
        <img :src="thumb" class="media" draggable="false" />
        <span class="gone-note"><AppIcon name="alert-triangle" :size="13" /> {{ item.status === 'Offline' ? $t('formats.preview.offline') : $t('formats.preview.missing') }}</span>
      </template>
      <div v-else-if="item.status !== 'Ok'" class="none">
        <AppIcon :name="KIND_ICON[item.kind] || 'file'" :size="compact ? 40 : 72" class="faint" />
        <span class="faint">{{ item.status === 'Offline' ? $t('formats.preview.offline') : $t('formats.preview.missing') }}</span>
      </div>
      <FontView v-else-if="kind === 'font' && !compact && item.status === 'Ok'" :item="item" />
      <LinkView v-else-if="kind === 'link'" :item="item" :compact="compact" />
      <VideoPlayer v-else-if="kind === 'video' && !compact" :item="item" :rotation="rotation" :flip="flip" />
      <AudioPlayer v-else-if="kind === 'audio' && !compact" :item="item" />
      <TextView v-else-if="kind === 'text'" :item="item" :compact="compact" />
      <iframe v-else-if="kind === 'pdf' && !compact" class="frame fills-stage" :src="originalUrl(id)" />
      <ImageView v-else-if="kind === 'image' && !(compact && isAnimated(item) && thumb) && !failed" :item="item" :zoom="zoom" :pan="pan" :compact="compact" :flip="flip" @failed="failed = true" />
      <img v-else-if="thumb" :src="thumb" class="media" draggable="false" />
      <div v-else class="none">
        <AppIcon :name="KIND_ICON[item.kind] || 'file'" :size="compact ? 40 : 72" class="faint" />
        <span v-if="kind !== 'audio' && kind !== 'video'" class="faint">{{ $t('formats.preview.none', { ext: item.ext }) }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.preview { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; }
.media { max-width: 100%; max-height: 100%; object-fit: contain; user-select: none; }
.compact .media { border-radius: calc(6px * var(--r)); }
.frame { width: 100%; height: 100%; border: 0; background: #fff; }
.none { display: flex; flex-direction: column; align-items: center; gap: calc(0.7rem * var(--g)); text-align: center; padding: calc(0.7rem * var(--g)); }
.gone-note { position: absolute; bottom: calc(0.56rem * var(--g)); display: inline-flex; align-items: center; gap: calc(0.35rem * var(--g)); padding: 0.14rem calc(0.56rem * var(--g)); border-radius: calc(4px * var(--r)); background: rgba(0, 0, 0, .6); color: #fff; font-size: 0.85rem; }
</style>
