<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { on } from '../platform/ipc.js'
import { S } from '../app/state.js'
import PluginFrame from './PluginFrame.vue'
import WindowControls from '../app/WindowControls.vue'
import MacWindowButtons from '../app/MacWindowButtons.vue'
import ApiAsk from '../app/ApiAsk.vue'
import PageOverlay from './PageOverlay.vue'
import TagFolderPicker from '../tags/TagFolderPicker.vue'
import TheToasts from '../app/TheToasts.vue'

// A Window plugin: the app's frame and title bar around the plugin's page, which asks its questions over it.
const frame = ref(null)
let off = null
// opened again for other items while open: the page hears plugin.run once more
onMounted(async () => {
  const me = getCurrentWindow().label
  off = await on('plugin-rerun', ({ to, run }) => to === me && frame.value?.run(run))
})
onBeforeUnmount(() => off?.())
</script>

<template>
  <div class="app plugin-window">
    <main class="center">
      <header class="pw-bar" :class="{ lights: S.lights }" data-window-drag>
        <MacWindowButtons v-if="S.lights" />
        <span class="pw-title ellipsis" data-window-drag>{{ S.pluginWindow.title }}</span>
      </header>
      <div class="pw-page"><PluginFrame ref="frame" :plugin="S.pluginWindow.plugin" :page="S.pluginWindow.page" :run="S.pluginWindow.run" /></div>
    </main>
    <TagFolderPicker />
    <ApiAsk />
    <PageOverlay />
    <WindowControls />
    <TheToasts />
  </div>
</template>

<style scoped>
.plugin-window { height: 100%; }
.center { display: flex; flex-direction: column; height: 100%; min-width: 0; flex: 1; background: var(--bg); }
.pw-bar { flex: none; height: round(2.8rem, 1px); display: flex; align-items: center; padding: 0 calc(var(--wc-width, 0px) + 0.56rem) 0 calc(0.98rem * var(--g)); }
/* level with the main window's toolbar, where a Mac's window buttons are centred */
.pw-bar.lights { height: round(3.36rem, 1px); padding-left: 0.7rem; }
.pw-title { font-weight: 500; color: var(--text-muted); }
.pw-page { flex: 1; min-height: 0; }
</style>
