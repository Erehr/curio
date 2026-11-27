<script setup>
import Chip from 'openvue/chip'
import AppIcon from './AppIcon.vue'

// OpenVue's Chip in Curio's look, its remove button a Tabler cross: inside the chip, or with `corner` a badge on its corner shown on hover.
defineProps({ label: { type: [String, Number], default: '' }, removable: Boolean, corner: Boolean })
const emit = defineEmits(['remove'])
</script>

<template>
  <Chip class="chip" :removable="removable" @remove="emit('remove')">
    <slot>{{ label }}</slot>
    <template #removeicon="{ removeCallback }">
      <button type="button" :class="corner ? 'chip-clear' : 'chip-x'" :aria-label="$t('shared.chip.remove')" @click.stop="removeCallback"><AppIcon name="x" :size="corner ? 10 : 12" /></button>
    </template>
  </Chip>
</template>
