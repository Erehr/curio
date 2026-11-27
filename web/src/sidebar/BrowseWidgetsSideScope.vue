<script setup>
import { S } from '../app/state.js'
import { LOOK_COLORS, colorName } from './sidebar.js'
import AppCheckboxTristate from '../shared/AppCheckboxTristate.vue'

// The sidebar filter's options: empty folders kept or left out, and the colours a folder may have.
function toggleColor(c) {
  const list = S.sideScope.colors
  const i = list.indexOf(c)
  if (i >= 0) list.splice(i, 1)
  else list.push(c)
}
</script>

<template>
  <div class="side-scope">
    <div class="section-title">{{ $t('sidebar.scope.folders') }}</div>
    <AppCheckboxTristate v-model="S.sideScope.empty" :label="$t('sidebar.scope.empty')" />
    <div class="section-title">{{ $t('sidebar.scope.color') }}</div>
    <div class="swatches">
      <button v-for="[n, c] in LOOK_COLORS" :key="n" class="swatch-dot" :class="{ sel: S.sideScope.colors.includes(c), none: !c }" :style="{ background: c || undefined }" :aria-label="colorName(n)" v-tooltip.bottom="colorName(n)" @click="toggleColor(c)" />
    </div>
  </div>
</template>

<style>
.side-scope { display: flex; flex-direction: column; gap: calc(0.56rem * var(--g)); width: round(16.78rem, 1px); font-size: 1rem; }
</style>
