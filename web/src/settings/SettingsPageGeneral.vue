<script setup>
import { computed } from 'vue'
import Select from 'openvue/select'
import Slider from 'openvue/slider'
import SelectButton from 'openvue/selectbutton'
import ToggleSwitch from 'openvue/toggleswitch'
import Tag from 'openvue/tag'
import { S } from '../app/state.js'
import { systemLanguage } from '../app/language.js'
import { languageName, languages, t } from '../shared/i18n.js'
import { useSettingsData, setUi } from './settingsData.js'

const { appCfg, libSettings, setCfg, saveLib } = useSettingsData()
const NAME_SUFFIXES = computed(() => [{ v: 'paren', l: `${t('settings.general.background.exampleName')} (2).png` }, { v: 'hash', l: `${t('settings.general.background.exampleName')} #2.png` }, { v: 'dash', l: `${t('settings.general.background.exampleName')} - 2.png` }])
const options = (keys, label) => keys.map((v) => ({ v, l: label(v) }))
const materialOptions = computed(() => options(['off', 'mica', 'acrylic'], (v) => t(`settings.general.appearance.materials.${v}`)).filter((m) => (S.startup.materials || []).includes(m.v)))
const ACRYLIC_OPACITY = computed(() => options(['low', 'medium', 'high'], (v) => t(`settings.general.appearance.transparencies.${v}`)))
// a Mac keeps a running program in its menu bar, and minimises to the Dock
const MAC = navigator.userAgent.includes('Mac')
const CLOSE_ACTIONS = computed(() => [{ v: true, l: MAC ? t('settings.general.startup.toTrayMac') : t('settings.general.startup.toTray') }, { v: false, l: t('settings.general.startup.quit') }])
const HASH_OPTIONS = computed(() => ['OnDemand', 'SmallFiles', 'Always'].map((v) => ({ v, l: t(`settings.general.hashing.policies.${v}`) })))
const ZOOM_LEVELS = [60, 70, 80, 90, 100, 110, 120, 135, 150, 175, 200].map((v) => ({ v, l: v + '%' }))
// listed once a second language is bundled, the system's first; the setting keeps it as '', which a Select shows as nothing chosen
const SYSTEM = 'system'
const languageOptions = computed(() => [{ v: SYSTEM, l: t('settings.general.language.system', { language: languageName(systemLanguage()) }) }, ...languages().map((c) => ({ v: c, l: languageName(c) }))])
</script>

<template>
  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.general.appearance.title') }}</span></div>
    <div class="s-card">
      <div v-if="languages().length > 1" class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.general.language.label') }}</span></div>
        <div class="s-ctrl w-md"><Select :modelValue="S.ui.language || SYSTEM" :options="languageOptions" optionLabel="l" optionValue="v" @update:modelValue="(v) => setUi('language', v === SYSTEM ? '' : v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.general.appearance.zoom') }}</span></div>
        <div class="s-ctrl w-sm"><Select :modelValue="S.ui.zoomLevel || 100" :options="ZOOM_LEVELS" optionLabel="l" optionValue="v" @update:modelValue="(v) => setUi('zoomLevel', v)" /></div>
      </div>
      <template v-if="materialOptions.length > 1">
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.appearance.material') }}</span></div>
          <div class="s-ctrl"><SelectButton :modelValue="S.ui.windowMaterial" :options="materialOptions" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('windowMaterial', v)" /></div>
        </div>
        <div v-if="S.ui.windowMaterial === 'acrylic'" class="s-row indented">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.appearance.transparency') }}</span></div>
          <div class="s-ctrl"><SelectButton :modelValue="S.ui.acrylicTransparency || 'medium'" :options="ACRYLIC_OPACITY" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('acrylicTransparency', v)" /></div>
        </div>
        <div v-if="S.ui.windowMaterial && S.ui.windowMaterial !== 'off'" class="s-row indented">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.appearance.keepMaterial') }}</span></div>
          <div class="s-ctrl"><ToggleSwitch :modelValue="!!S.ui.keepMaterial" :aria-label="$t('settings.general.appearance.keepMaterial')" @update:modelValue="(v) => setUi('keepMaterial', v)" /></div>
        </div>
      </template>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.general.appearance.sidebarCounts') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="S.ui.sidebarCounts !== false" :aria-label="$t('settings.general.appearance.sidebarCounts')" @update:modelValue="(v) => setUi('sidebarCounts', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.general.appearance.windowButtons') }}</span>
          <span class="s-desc">{{ $t('settings.general.appearance.windowButtonsOff') }}</span>
        </div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="S.ui.windowButtons !== false" :aria-label="$t('settings.general.appearance.windowButtons')" @update:modelValue="(v) => setUi('windowButtons', v)" /></div>
      </div>
    </div>
  </section>

  <section v-if="appCfg" class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.general.startup.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.general.startup.autostart') }}</span>
          <span v-if="appCfg.portable" class="s-desc">{{ $t('settings.general.startup.portable') }}</span>
        </div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.autostart" :aria-label="$t('settings.general.startup.autostart')" @update:modelValue="(v) => setCfg({ autostart: v })" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.general.startup.onClose') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="appCfg.closeToTray" :options="CLOSE_ACTIONS" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setCfg({ closeToTray: v })" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.general.startup.gpu') }} <Tag severity="warn" :value="$t('settings.general.startup.restart')" /></span></div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.gpu" :aria-label="$t('settings.general.startup.gpu')" @update:modelValue="(v) => setCfg({ gpu: v })" /></div>
      </div>
    </div>
  </section>

  <div v-if="!libSettings" class="s-card s-empty">{{ $t('settings.general.noLibrary') }}</div>
  <template v-else>
    <section class="s-section">
      <div class="s-section-head">
        <span class="s-section-title">{{ $t('settings.general.indexing.title') }}</span>
      </div>
      <div class="s-card">
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.indexing.allTypes') }}</span></div>
          <div class="s-ctrl"><ToggleSwitch v-model="libSettings.index_all_files" :aria-label="$t('settings.general.indexing.allTypes')" @change="saveLib" /></div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.indexing.watch') }}</span></div>
          <div class="s-ctrl"><ToggleSwitch v-model="libSettings.watch_folders" :aria-label="$t('settings.general.indexing.watch')" @change="saveLib" /></div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.hashing.title') }}</span></div>
          <div class="s-ctrl"><SelectButton v-model="libSettings.hash_policy" :options="HASH_OPTIONS" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveLib" /></div>
        </div>
        <div v-if="libSettings.hash_policy === 'SmallFiles'" class="s-row indented">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.hashing.limitLabel') }}</span></div>
          <div class="s-ctrl">
            <div class="s-slider-wrap">
              <Slider v-model="libSettings.hash_small_limit_mb" :min="1" :max="2048" @slideend="saveLib" />
              <span class="s-slider-val">{{ $t('settings.general.hashing.mb', { size: libSettings.hash_small_limit_mb }) }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="s-section">
      <div class="s-section-head"><span class="s-section-title">{{ $t('settings.general.background.title') }}</span></div>
      <div class="s-card">
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.background.workersLabel') }} <Tag severity="secondary" :value="$t('settings.general.background.onReopen')" /></span></div>
          <div class="s-ctrl">
            <div class="s-slider-wrap">
              <Slider v-model="libSettings.thumb_workers" :min="1" :max="16" @slideend="saveLib" />
              <span class="s-slider-val">{{ libSettings.thumb_workers }}</span>
            </div>
          </div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.background.maintenanceLabel') }}</span></div>
          <div class="s-ctrl">
            <div class="s-slider-wrap">
              <Slider v-model="libSettings.maintenance_minutes" :min="5" :max="240" @slideend="saveLib" />
              <span class="s-slider-val">{{ $t('settings.general.background.minutes', { minutes: libSettings.maintenance_minutes }) }}</span>
            </div>
          </div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.general.background.duplicateNames') }}</span></div>
          <div class="s-ctrl w-md"><Select :modelValue="libSettings.name_suffix || 'paren'" :options="NAME_SUFFIXES" optionLabel="l" optionValue="v" @update:modelValue="(v) => { libSettings.name_suffix = v; saveLib() }" /></div>
        </div>
      </div>
    </section>
  </template>
</template>

<style scoped src="./settings.css"></style>
