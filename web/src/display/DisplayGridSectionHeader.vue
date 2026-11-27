<script setup>
import AppIcon from '../shared/AppIcon.vue'

// Collapsible section header shared by Subfolders, Contents and Group by sections.
defineProps({ title: String, count: Number, collapsed: Boolean })
const emit = defineEmits(['toggle', 'contextmenu'])
</script>

<template>
  <div class="sec-head" @pointerdown.stop @contextmenu.prevent.stop="emit('contextmenu', $event)">
    <button type="button" class="sec-toggle" :aria-expanded="!collapsed" @click="emit('toggle')">
      <AppIcon :name="collapsed ? 'chevron-right' : 'chevron-down'" :size="15" class="sec-chev" />
      <span class="sec-title ellipsis">{{ title }}</span>
      <span class="sec-count">{{ count.toLocaleString() }}</span>
    </button>
    <div v-if="$slots.right" class="sec-right"><slot name="right" /></div>
  </div>
</template>

<style scoped>
.sec-head { position: absolute; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.38rem, 1px); font-size: 1rem; color: var(--text-muted); user-select: none; }
.sec-toggle { display: inline-flex; align-items: center; gap: calc(0.35rem * var(--g)); min-width: 0; max-width: 75%; padding: calc(0.14rem * var(--g)) calc(0.28rem * var(--g)) calc(0.14rem * var(--g)) 0; border: 0; border-radius: calc(5px * var(--r)); background: transparent; color: inherit; font: inherit; cursor: pointer; }
.sec-toggle:hover { color: var(--text); }
.sec-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.sec-chev { flex: none; opacity: 0.8; }
.sec-title { font-weight: 500; color: var(--text); }
.sec-count { flex: none; font-size: 0.85rem; color: var(--text-faint); }
.sec-right { margin-left: auto; flex: none; display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); }
</style>
