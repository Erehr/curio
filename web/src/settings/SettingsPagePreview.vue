<script setup>
import { computed } from 'vue'
import SelectButton from 'openvue/selectbutton'
import ToggleSwitch from 'openvue/toggleswitch'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { useSettingsData, setUi } from './settingsData.js'
import { t } from '../shared/i18n.js'

const { appCfg, setCfg } = useSettingsData()
const STRIP_SIDES = computed(() => ['left', 'bottom', 'right'].map((v) => ({ v, l: t(`settings.preview.image.strip.${v}`) })))
const UPSCALING = computed(() => [{ v: true, l: t('settings.preview.image.pixelated') }, { v: false, l: t('settings.preview.image.smooth') }])
const DEFAULT_ZOOM = computed(() => [{ v: 'fit', l: t('settings.preview.image.fit') }, { v: 'original', l: t('settings.preview.image.original') }])
const ROTATION = computed(() => [{ v: 'preview', l: t('settings.preview.image.viewOnly') }, { v: 'file', l: t('settings.preview.image.saveToFile') }])
const TRANSPARENCY = computed(() => [{ v: true, l: t('settings.preview.image.checkerboard') }, { v: false, l: t('settings.preview.image.solid') }])
const VIDEO_WHEEL = computed(() => [{ v: 'seek', l: t('settings.preview.video.seek') }, { v: 'volume', l: t('settings.preview.video.volume') }, { v: 'navigate', l: t('settings.preview.video.navigate') }])
</script>

<template>
  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.preview.image.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.filmstrip') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="S.ui.viewerStripSide || 'bottom'" :options="STRIP_SIDES" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('viewerStripSide', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.upscaling') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="!!S.ui.pixelated" :options="UPSCALING" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('pixelated', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.zoom') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.defaultImageSize" :options="DEFAULT_ZOOM" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.rotation') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.imageRotation" :options="ROTATION" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.transparency') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="!!S.ui.transparency" :options="TRANSPARENCY" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('transparency', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.image.rememberZoom') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="!!S.ui.rememberZoom" :aria-label="$t('settings.preview.image.rememberZoom')" @update:modelValue="(v) => setUi('rememberZoom', v)" /></div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.preview.video.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.video.wheel') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.videoWheel" :options="VIDEO_WHEEL" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.video.hoverPlay') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.hoverPlay" :aria-label="$t('settings.preview.video.hoverPlay')" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.video.stretch') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.videoStretch" :aria-label="$t('settings.preview.video.stretch')" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.video.autoplay') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.videoAutoplay" :aria-label="$t('settings.preview.video.autoplay')" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.preview.video.resume') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.videoRememberPosition" :aria-label="$t('settings.preview.video.resume')" @change="saveUi" /></div>
      </div>
    </div>
  </section>

  <section v-if="appCfg" class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.preview.links.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.preview.links.previews') }}</span>
          <span class="s-desc">{{ $t('settings.preview.links.hint') }}</span>
        </div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.linkPreviews" :aria-label="$t('settings.preview.links.previews')" @update:modelValue="(v) => setCfg({ linkPreviews: v })" /></div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
