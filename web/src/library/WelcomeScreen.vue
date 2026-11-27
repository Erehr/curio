<script setup>
import logo from '../../icons/logo.svg'
import { S } from '../app/state.js'
import { t } from '../shared/i18n.js'
import { afterOpen } from '../app/boot.js'
import { run } from '../app/notify.js'
import AppIcon from '../shared/AppIcon.vue'

function createLibrary() {
  S.dialogs.libName = { mode: 'new', name: t('sidebar.newLibraryName') }
}
async function openExisting() {
  if (await run('open_library_dialog')) await afterOpen()
}
</script>

<template>
  <div class="welcome">
    <img :src="logo" class="welcome-logo" alt="" />
    <h1>Curio</h1>
    <p class="muted">{{ $t('library.welcome.about') }}</p>
    <button class="primary" @click="createLibrary"><AppIcon name="library-plus" :size="18" /> {{ $t('library.welcome.create') }}</button>
    <button class="secondary" @click="openExisting"><AppIcon name="folder-open" :size="18" /> {{ $t('library.welcome.open') }}</button>
    <p class="faint small">{{ $t('library.welcome.drop') }} {{ S.startup.safeMode ? $t('library.welcome.safeMode') : '' }}</p>
  </div>
</template>

<style scoped>
.welcome-logo { display: block; width: round(5.03rem, 1px); height: round(5.03rem, 1px); margin: 0 auto calc(0.84rem * var(--g)); }
.welcome { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: calc(0.84rem * var(--g)); }
h1 { margin: calc(0.28rem * var(--g)) 0 0; font-weight: 600; }
button { display: inline-flex; align-items: center; justify-content: center; gap: calc(0.56rem * var(--g)); width: round(20.98rem, 1px); height: round(2.66rem, 1px); border-radius: calc(8px * var(--r)); border: 1px solid var(--border); font: inherit; cursor: pointer; }
.primary { background: var(--accent); color: var(--accent-text); border-color: transparent; font-weight: 600; }
.secondary { background: var(--button); color: var(--text); }
.secondary:hover { background: var(--button-hover); }
.small { font-size: 0.92rem; }
</style>
