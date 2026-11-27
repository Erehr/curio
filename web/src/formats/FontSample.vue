<script setup>
import { computed, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { fontFamily } from './fontFaces.js'
import { t } from '../shared/i18n.js'

// A list cell writing the window's sample text in the item's own font, sized by the row, so the list's size control scales it; other items' cells stay empty.
const props = defineProps({ item: Object, height: Number })
const keyOf = (it) => (it?.kind === 'font' ? `${it.id}:${it.mtime}:${it.size}` : null)
const family = ref(null)
watch(
  () => keyOf(props.item),
  async (key) => {
    family.value = null
    if (!key) return
    const f = await fontFamily(props.item.id, key)
    if (key === keyOf(props.item)) family.value = f
  },
  { immediate: true },
)
const text = computed(() => S.ui.fontSample?.text || t('formats.fontSample'))
const size = computed(() => Math.max(8, Math.round((props.height || 40) * 0.6)))
</script>

<template>
  <span v-if="family" class="fsample" :style="{ fontFamily: `'${family}'`, fontSize: size + 'px' }">{{ text }}</span>
</template>

<style scoped>
.fsample { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; line-height: 1.2; color: var(--text); font-weight: normal; font-style: normal; }
</style>
