// The language Curio's text is shown in, one vue-i18n instance per page, made from the catalogs its build passes in (locales/, docs in dev/docs/translating.md).
import { createI18n } from 'vue-i18n'
import { pluralRule } from './plurals.js'

let i18n = null
let openvueText = {}
const openvueApps = new Set()

/** Makes the page's instance: `messages` and `openvue` (OpenVue's own text) by language code, starting in `language`. */
export function makeI18n({ messages, openvue = {}, language = 'en', warn = false }) {
  openvueText = openvue
  i18n = createI18n({
    legacy: false,
    globalInjection: true,
    locale: language in messages ? language : 'en',
    fallbackLocale: 'en',
    messages,
    pluralRules: Object.fromEntries(Object.keys(messages).map((c) => [c, pluralRule(c)])),
    missingWarn: warn,
    fallbackWarn: false,
  })
  if (globalThis.document) document.documentElement.lang = i18n.global.locale.value
  return i18n
}

/** The text of `key`, with its named values; a plural message takes its form from `plural` when given (a count shown formatted), else from `count`. */
export const t = (key, named, plural) => (i18n ? (plural === undefined ? i18n.global.t(key, named ?? {}) : i18n.global.t(key, named ?? {}, plural)) : key)

/** Whether English, which holds every key, has `key`. */
export const hasText = (key) => !!i18n?.global.te(key, 'en')

/** The English text of `key`, null when English has no such key. */
export const englishText = (key) => (i18n?.global.te(key, 'en') ? i18n.global.t(key, {}, { locale: 'en' }) : null)

/** The language shown now. */
export const language = () => i18n?.global.locale.value || 'en'

/** The languages this build carries, by code. */
export const languages = () => (i18n ? i18n.global.availableLocales : ['en'])

/** A language's name for itself, from its catalog. */
export const languageName = (code) => (i18n?.global.te('meta.name', code) ? i18n.global.t('meta.name', {}, { locale: code }) : code)

/** Adds a page's own messages (a core plugin's catalog) for `code`. */
export const addMessages = (code, messages) => i18n?.global.mergeLocaleMessage(code, messages)

/** Shows every text of the page, OpenVue's in each of its apps included, in `code` when this build carries it, else in English. */
export function setLanguage(code) {
  if (!i18n) return
  const next = i18n.global.availableLocales.includes(code) ? code : 'en'
  i18n.global.locale.value = next
  document.documentElement.lang = next
  for (const app of openvueApps) applyOpenVue(app)
}

/** OpenVue's own text (day names, empty lists, aria labels) for an app that uses it, kept in the page's language from now on. */
export function followLanguage(app) {
  openvueApps.add(app)
  app.onUnmount(() => openvueApps.delete(app))
  applyOpenVue(app)
}

// a language's file may leave out what it doesn't change: it is laid over English, its aria labels too
function applyOpenVue(app) {
  const config = app.config.globalProperties.$primevue?.config
  if (!config) return
  const en = openvueText.en || {}
  const own = openvueText[language()] || {}
  config.locale = { ...config.locale, ...en, ...own, aria: { ...config.locale?.aria, ...en.aria, ...own.aria } }
}
