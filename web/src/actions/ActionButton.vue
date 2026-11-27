<script setup>
import { ref } from 'vue'
import { t } from '../shared/i18n.js'
import { S } from '../app/state.js'
import { withKey } from '../app/shortcuts.js'
import { runUserAction, saveActions } from './actions.js'
import AppIcon from '../shared/AppIcon.vue'
import AppMenu from '../shared/AppMenu.vue'

// The lightning button: run an Action on the selection, or make one.
const menu = ref()
// by id: coming back to the window loads the actions again while the menu may still be open
function change(a, fn) {
  const i = S.actions.findIndex((x) => x.id === a.id)
  if (i < 0) return
  fn(i, S.actions[i])
  saveActions()
}
const options = (a) => [
  { label: t('common.edit'), icon: 'pencil', command: () => (S.dialogs.action = a) },
  { label: t('common.duplicate'), icon: 'copy-plus', command: () => change(a, (i, x) => S.actions.splice(i + 1, 0, { ...JSON.parse(JSON.stringify(x)), id: 'a' + Date.now().toString(36), name: t('actions.copyName', { name: x.name }), shortcut: '' })) },
  { separator: true },
  { label: t('common.remove'), icon: 'trash', command: () => change(a, (i) => S.actions.splice(i, 1)) },
]
const items = () => [
  ...S.actions.map((a) => ({ label: a.name, icon: a.icon || 'bolt', shortcut: a.shortcut ? a.shortcut.split('+').join(' + ') : '', command: () => runUserAction(a), more: options(a) })),
  { separator: true },
  { label: t('actions.new'), icon: 'plus', command: () => (S.dialogs.action = { create: true }) },
]
</script>

<template>
  <button class="icon-btn" data-action-btn v-tooltip.bottom="withKey($t('actions.button'), 'item.actions')" @click="menu.toggle($event)"><AppIcon name="bolt" :size="18" /></button>
  <AppMenu ref="menu" :model="items" />
</template>
