<script setup>
import { onBeforeUnmount, onMounted } from 'vue'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { on } from '../platform/ipc.js'
import { S } from '../app/state.js'
import WindowControls from '../app/WindowControls.vue'
import MacWindowButtons from '../app/MacWindowButtons.vue'
import ApiAsk from '../app/ApiAsk.vue'
import PageOverlay from '../plugins/PageOverlay.vue'
import TagFolderPicker from '../tags/TagFolderPicker.vue'
import TheToasts from '../app/TheToasts.vue'
import DuplicatesWindow from './DuplicatesWindow.vue'
import RenameWindow from './RenameWindow.vue'
import CompareWindow from './CompareWindow.vue'

// One of Curio's tools: the app's frame and title bar around it, its questions asked over it.
const TOOL = { duplicates: DuplicatesWindow, rename: RenameWindow, compare: CompareWindow }
let off = null
// opened again for other items while open: the tool takes the new run
onMounted(async () => {
  const me = getCurrentWindow().label
  off = await on('tool-rerun', ({ to, run }) => {
    if (to !== me) return
    S.toolWindow.run = run || {}
    S.toolWindow.seq++
  })
})
onBeforeUnmount(() => off?.())
</script>

<template>
  <div class="app tool-window">
    <main class="center">
      <header class="tw-bar" :class="{ lights: S.lights }" data-window-drag>
        <MacWindowButtons v-if="S.lights" />
        <span class="tw-title ellipsis" data-window-drag>{{ $t(`tools.titles.${S.toolWindow.tool}`) }}</span>
      </header>
      <div class="tw-page"><component :is="TOOL[S.toolWindow.tool]" v-if="TOOL[S.toolWindow.tool]" /></div>
    </main>
    <TagFolderPicker />
    <ApiAsk />
    <PageOverlay />
    <WindowControls />
    <TheToasts />
  </div>
</template>

<style scoped>
.tool-window { height: 100%; }
.center { display: flex; flex-direction: column; height: 100%; min-width: 0; flex: 1; background: var(--bg); }
.tw-bar { flex: none; height: round(2.8rem, 1px); display: flex; align-items: center; padding: 0 calc(var(--wc-width, 0px) + 0.56rem) 0 calc(0.98rem * var(--g)); }
/* level with the main window's toolbar, where a Mac's window buttons are centred */
.tw-bar.lights { height: round(3.36rem, 1px); padding-left: 0.7rem; }
.tw-title { font-weight: 500; color: var(--text-muted); }
.tw-page { flex: 1; min-height: 0; }
</style>

<style src="./tools.css"></style>
