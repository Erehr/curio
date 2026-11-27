<script setup>
import { computed } from 'vue'
import { matchParts } from './text.js'

// A name with the part matching the search phrase highlighted.
const props = defineProps({ text: String, q: String })
const parts = computed(() => matchParts(props.text || '', props.q || ''))
</script>

<template><template v-for="(p, i) in parts" :key="i"><b v-if="p.hit" class="hl">{{ p.t }}</b><template v-else>{{ p.t }}</template></template></template>

<style>
b.hl { color: var(--accent); font-weight: inherit; }
</style>
