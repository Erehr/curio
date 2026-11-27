// Key names as shown: German Windows writes "Strg" and "Entf". Chords are stored with English names (app/shortcuts.js), and only their display goes through the catalog.
import { hasText, t } from './i18n.js'

// a Mac's keycaps carry symbols, and its Command key does what Ctrl does elsewhere (app/shortcuts.js stores both as Ctrl)
const MAC = navigator.userAgent.includes('Mac')

/** One key's name, as its keycap reads. */
export const keyName = (key) => (MAC && hasText(`shared.keysMac.${key}`) ? t(`shared.keysMac.${key}`) : hasText(`shared.keys.${key}`) ? t(`shared.keys.${key}`) : key)

// the order a Mac's menus write modifiers in: Option, Shift, Command
const MAC_ORDER = ['Alt', 'Shift', 'Ctrl']

/** A chord such as "Ctrl+Shift+Delete" as shown: "Ctrl + Shift + Delete", or "⇧⌘⌦" on a Mac as its menus write it; `joiner` puts its own text between the keys. */
export const keyText = (chord, joiner) => {
  if (!chord) return ''
  const keys = chord.split('+').map((k) => k.trim())
  if (MAC && joiner == null) return [...MAC_ORDER.filter((m) => keys.includes(m)), ...keys.filter((k) => !MAC_ORDER.includes(k))].map(keyName).join('')
  return keys.map(keyName).join(joiner ?? ' + ')
}
