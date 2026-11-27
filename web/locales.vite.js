// The catalogs of locales/ for a build, as `virtual:curio-locales`: each file it asks for, by language code. vue-i18n's plugin precompiles the message files; OpenVue's text stays plain JSON in OpenVue's own shape.
// A debug build adds the pseudo-language, made from English into target/pseudo-locales.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite'
import { PSEUDO, pseudoCatalog } from './src/shared/pseudoLanguage.js'

const posix = (p) => p.replace(/\\/g, '/')
const ROOT = posix(fileURLToPath(new URL('../locales/', import.meta.url)))
const PSEUDO_ROOT = posix(fileURLToPath(new URL('../target/pseudo-locales/', import.meta.url)))
const MESSAGES = ['app']
// OpenVue's text has a pseudo version too, with its date format kept, so its own labels show they follow the language
const PSEUDO_FILES = [...MESSAGES, 'openvue']
const ID = 'virtual:curio-locales'

/** The languages in locales/, English first. */
export const codes = () => readdirSync(ROOT, { withFileTypes: true }).filter((e) => e.isDirectory()).map((e) => e.name).sort((a, b) => (a === 'en' ? -1 : b === 'en' ? 1 : a.localeCompare(b)))

function writePseudo(files) {
  mkdirSync(`${PSEUDO_ROOT}${PSEUDO}`, { recursive: true })
  for (const f of files.filter((f) => PSEUDO_FILES.includes(f))) {
    const en = JSON.parse(readFileSync(`${ROOT}en/${f}.json`, 'utf8'))
    const messages = pseudoCatalog(en)
    if (f === 'openvue') messages.dateFormat = en.dateFormat
    if (f === 'app') messages.meta = { name: `Pseudo (${PSEUDO})` }
    writeFileSync(`${PSEUDO_ROOT}${PSEUDO}/${f}.json`, JSON.stringify(messages, null, 2) + '\n')
  }
}

/** `files`: which of a language's files the build carries; `runtimeOnly`: vue-i18n without its message compiler (which `@intlify/core-base` brings in unless dropped too), every message precompiled. */
export function locales({ files, pseudo, runtimeOnly }) {
  const catalogs = {
    name: 'curio-locales',
    resolveId: (id) => (id === ID ? '\0' + ID : null),
    load(id) {
      if (id !== '\0' + ID) return null
      if (pseudo) writePseudo(files)
      const imports = []
      const exports = []
      for (const f of files) {
        const entries = []
        for (const [code, dir] of [...codes().map((c) => [c, ROOT]), ...(pseudo && PSEUDO_FILES.includes(f) ? [[PSEUDO, PSEUDO_ROOT]] : [])]) {
          const path = `${dir}${code}/${f}.json`
          if (!existsSync(path)) continue
          this.addWatchFile(path)
          imports.push(`import m${imports.length} from ${JSON.stringify(path)}`)
          entries.push(`${JSON.stringify(code)}: m${imports.length - 1}`)
        }
        exports.push(`export const ${f} = { ${entries.join(', ')} }`)
      }
      return [...imports, ...exports].join('\n')
    },
  }
  return [VueI18nPlugin({ include: [...MESSAGES.flatMap((f) => [`${ROOT}*/${f}.json`, `${PSEUDO_ROOT}*/${f}.json`])], runtimeOnly, dropMessageCompiler: runtimeOnly, compositionOnly: true, fullInstall: true, strictMessage: true }), catalogs]
}
