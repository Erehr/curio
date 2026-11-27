<script setup>
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from './state.js'
import AppIcon from '../shared/AppIcon.vue'

// A Mac's close, minimise and zoom, drawn by Curio in the system's colours where the window's layout puts them (App.vue).
const win = getCurrentWindow()
</script>

<template>
  <div class="mac-buttons">
    <button class="mb close" tabindex="-1" :aria-label="$t('app.windowControls.close')" @click="win.close()"><AppIcon name="x" :size="8" /></button>
    <button class="mb min" tabindex="-1" :aria-label="$t('app.windowControls.minimize')" @click="win.minimize()"><AppIcon name="minus" :size="8" /></button>
    <button class="mb zoom" tabindex="-1" :aria-label="$t('app.windowControls.maximize')" @click="win.toggleMaximize()"><AppIcon :name="S.maximized ? 'arrows-diagonal-minimize-2' : 'arrows-diagonal'" :size="8" /></button>
  </div>
</template>

<style scoped>
.mac-buttons { display: flex; flex: none; align-items: center; gap: round(0.62rem, 1px); margin: 0 round(0.7rem, 1px) 0 round(0.38rem, 1px); }
.mb { display: flex; align-items: center; justify-content: center; width: round(0.92rem, 1px); height: round(0.92rem, 1px); padding: 0; border: 0; border-radius: 50%; color: transparent; cursor: default; }
.mb.close { background: #ff5f57; }
.mb.min { background: #febc2e; }
.mb.zoom { background: #28c840; }
.mac-buttons:hover .mb { color: rgb(0 0 0 / 0.6); }
:root.fx-inactive .mac-buttons:not(:hover) .mb { background: var(--border); }
</style>
