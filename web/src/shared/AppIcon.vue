<script setup>
import { Icon } from '@iconify/vue'
import { iconSize } from './rem.js'
const props = defineProps({ name: { type: String, required: true }, size: { type: [Number, String], default: 16 } })
const full = () => (props.name.includes(':') ? props.name : `tabler:${props.name}`)
</script>

<template>
  <Icon :icon="full()" :style="{ width: iconSize(size), height: iconSize(size) }" class="ico" />
</template>
