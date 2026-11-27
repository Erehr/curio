<script setup>
import Rating from 'openvue/rating'
import AppIcon from './AppIcon.vue'

// OpenVue's Rating with Tabler stars; it answers null when the current star is clicked again.
const model = defineModel({ type: Number, default: 0 })
defineProps({ readonly: Boolean, size: { type: Number, default: 16 } })
</script>

<template>
  <Rating v-model="model" class="curio-rating" :readonly="readonly">
    <template #onicon><AppIcon name="star-filled" :size="size" /></template>
    <template #officon><AppIcon name="star-filled" :size="size" /></template>
  </Rating>
</template>
