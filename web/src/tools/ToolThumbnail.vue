<script setup>
import { computed } from 'vue'
import { thumbUrl } from '../platform/fileUrls.js'
import AppIcon from '../shared/AppIcon.vue'
import { rows } from './tools.js'

// An item's thumbnail by its lasting id, as the grid shows it; its row comes from loadRows.
const props = defineProps({ item: { type: String, required: true }, size: { type: Number, default: 40 } })
const row = computed(() => rows.get(props.item))
const url = computed(() => (row.value?.thumb_state === 1 ? thumbUrl(row.value.uid, row.value.thumb_ver) : ''))
</script>

<template>
  <div class="tool-thumb" :style="{ width: `${size}px`, height: `${size}px` }">
    <img v-if="url" :src="url" alt="" draggable="false" />
    <AppIcon v-else name="photo" :size="Math.min(28, size / 2)" />
  </div>
</template>
