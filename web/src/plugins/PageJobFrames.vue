<script setup>
import { PJ, setFrameRef, pageAnswered } from './pageJobs.js'
import PluginFrame from './PluginFrame.vue'

// The pages making thumbnails and reading files, drawn off screen at a real size so WebGL and layout work as in a visible page.
</script>

<template>
  <div class="page-jobs" aria-hidden="true">
    <div v-for="f in PJ.frames" :key="f.key" class="pj-frame">
      <PluginFrame :ref="(c) => setFrameRef(f.key, c)" :plugin="f.plugin" :page="f.page" :role="f.role" :visible="false" :on-answer="pageAnswered" />
    </div>
  </div>
</template>

<style scoped>
.page-jobs { position: fixed; left: -10000px; top: 0; pointer-events: none; }
.pj-frame { width: 800px; height: 800px; }
</style>
