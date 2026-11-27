<script setup>
import AppIcon from './AppIcon.vue'

// The heading of an Inspector widget, Curio's or a plugin's: its name, folding it on a click, with room for its own buttons after.
defineProps({ title: String, collapsed: Boolean, foldable: { type: Boolean, default: true } })
const emit = defineEmits(['toggle'])
</script>

<template>
  <div class="w-head">
    <span class="sec-toggle" @click="foldable && emit('toggle')"><span>{{ title }}</span><AppIcon v-if="foldable" :name="collapsed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" />
    <slot />
  </div>
</template>

<style scoped>
.w-head { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); padding: calc(0.56rem * var(--g)) 0 calc(0.42rem * var(--g)); font-size: 0.92rem; font-weight: 500; color: var(--text-faint); cursor: default; }
.sec-toggle { display: inline-flex; align-items: center; gap: calc(0.35rem * var(--g)); height: round(1.68rem, 1px); min-width: 0; margin-left: calc(-0.42rem * var(--g)); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); white-space: nowrap; }
.sec-toggle:hover { background: var(--hover); color: var(--text); }
.sec-caret { visibility: hidden; color: var(--text-muted); }
.sec-toggle:hover .sec-caret { visibility: visible; }
</style>
