<script setup>
import { computed } from 'vue'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import Select from 'openvue/select'
import ToggleSwitch from 'openvue/toggleswitch'
import AppIcon from '../shared/AppIcon.vue'
import { S } from '../app/state.js'
import { playNotifySound } from '../app/notify.js'
import { t } from '../shared/i18n.js'
import { setUi } from './settingsData.js'

const soundOn = computed(() => S.ui.notifySound !== false)
const SOUND_EVENTS = ['import', 'deleteItem', 'deleteFolder', 'error', 'other']
const setSoundEvent = (key, v) => setUi('notifySoundEvents', { ...(S.ui.notifySoundEvents || {}), [key]: v })
const ADD_NOTICES = computed(() => ['all', 'background', 'failures', 'off'].map((v) => ({ v, l: t(`settings.notifications.system.when.${v}`) })))
</script>

<template>
  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.notifications.sound.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.notifications.sound.enable') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="soundOn" :aria-label="$t('settings.notifications.sound.enable')" @update:modelValue="(v) => setUi('notifySound', v)" /></div>
      </div>
      <div class="s-subblock sound-sub" :class="{ disabled: !soundOn }">
        <div class="s-check-grid sound-grid">
          <label v-for="k in SOUND_EVENTS" :key="k" class="opt"><Checkbox :modelValue="!!S.ui.notifySoundEvents?.[k]" binary :disabled="!soundOn" @update:modelValue="(v) => setSoundEvent(k, v)" /> {{ $t(`settings.notifications.sound.events.${k}`) }}</label>
          <label class="opt"><Checkbox :modelValue="S.ui.notifySoundOs !== false" binary :disabled="!soundOn" @update:modelValue="(v) => setUi('notifySoundOs', v)" /> {{ $t('settings.notifications.sound.os') }}</label>
        </div>
        <div class="s-actions">
          <Button :label="$t('settings.notifications.sound.test')" severity="secondary" size="small" :disabled="!soundOn" @click="playNotifySound(true)"><template #icon><AppIcon name="volume" :size="15" /></template></Button>
        </div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.notifications.system.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.notifications.system.imports') }}</span>
          <span class="s-desc">{{ $t('settings.notifications.system.backgroundHint') }}</span>
        </div>
        <div class="s-ctrl w-md"><Select :modelValue="S.ui.addNotices || 'background'" :options="ADD_NOTICES" optionLabel="l" optionValue="v" @update:modelValue="(v) => setUi('addNotices', v)" /></div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.sound-sub { gap: calc(1.05rem * var(--g)); padding-top: calc(0.98rem * var(--g)); padding-bottom: calc(0.98rem * var(--g)); }
.sound-grid { gap: calc(0.91rem * var(--g)) calc(1.54rem * var(--g)); }
</style>
