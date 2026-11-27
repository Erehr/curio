// The language the window shows: the one chosen in Settings › General, else the system's, which Rust works out for every window and the tray alike (src/desktop/locale.rs).
import { watch } from 'vue'
import { call } from '../platform/ipc.js'
import { S } from './state.js'
import { languages, setLanguage } from '../shared/i18n.js'
import { retranslatePlugins } from '../sidebar/sidebar.js'

let system = 'en'

/** Takes the language in force from Rust, before the window first draws. */
export async function startLanguage() {
  const got = await call('language').catch(() => null)
  if (got?.system) system = got.system
  setLanguage(got?.current || system)
}

/** The language an empty setting follows. */
export const systemLanguage = () => system

/** Keeps the window's language in step with the setting, changed here or in another window. */
export function followSetting() {
  watch(() => S.ui.language, (v) => {
    setLanguage(v && languages().includes(v) ? v : system)
    retranslatePlugins()
  })
}
