<script setup>
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Button from 'openvue/button'
import Tag from 'openvue/tag'
import AppIcon from '../shared/AppIcon.vue'
import { S } from '../app/state.js'
import { toast } from '../app/notify.js'
import { t } from '../shared/i18n.js'
import logo from '../../icons/logo.svg'

async function copyDir() {
  if (!S.startup.configDir) return
  await navigator.clipboard.writeText(S.startup.configDir)
  toast(t('settings.about.pathCopied'))
}
</script>

<template>
  <div class="s-card about-hero">
    <img :src="logo" class="about-logo" alt="" />
    <div class="s-meta">
      <span class="about-title">
        <span>{{ $t('settings.about.appName') }}</span>
        <Tag severity="info" :value="S.startup.version" />
        <Tag v-if="S.startup.portable" severity="secondary" :value="$t('settings.about.portable')" />
      </span>
      <span class="s-desc">{{ $t('settings.about.tagline') }}</span>
    </div>
  </div>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.about.detailsTitle') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.about.configDir') }}</span></div>
        <div class="s-ctrl w-lg">
          <InputGroup>
            <InputText class="s-mono" :modelValue="S.startup.configDir" :title="S.startup.configDir" readonly />
            <Button severity="secondary" size="small" :aria-label="$t('settings.api.tokens.copy')" @click="copyDir"><template #icon><AppIcon name="copy" :size="15" /></template></Button>
          </InputGroup>
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.about.stackLabel') }}</span>
          <span class="s-desc">{{ $t('settings.about.built') }}</span>
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.about.creditsLabel') }}</span>
          <span class="s-desc">{{ $t('settings.about.credits') }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.about-hero { display: flex; align-items: center; gap: calc(1.05rem * var(--g)); padding: calc(1.12rem * var(--g)) calc(1.26rem * var(--g)); }
.about-logo { display: block; width: round(3.36rem, 1px); height: round(3.36rem, 1px); flex: none; }
.about-title { display: flex; align-items: center; gap: calc(0.49rem * var(--g)); font-size: 1.18rem; font-weight: 600; flex-wrap: wrap; }
</style>
