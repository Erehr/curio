// Keyboard shortcuts;
// users can rebind any action in Settings → Shortcuts (stored in S.ui.keys).
import { S } from './state.js'
import { t } from '../shared/i18n.js'
import { keyName } from '../shared/keys.js'

// Defaults follow common desktop conventions: Ctrl+B side bars (code editors), Ctrl+Shift+digit view modes (file managers), Ctrl+Shift+L filters (spreadsheets), [ ] speed and , . frames (media players), R and H turn and mirror (image viewers).
// Ctrl+Alt with a letter is AltGr on many keyboards (ł, ń, ó, ś on Polish ones), so actions that work while typing avoid it.
export const ACTIONS = [
  // [id, group, default, works while typing]; the names are `shortcuts.actions.<id>` and `shortcuts.groups.<group>` in the catalogs
  ['panel.left', 'view', 'Ctrl+B', true],
  ['panel.right', 'view', 'Ctrl+Alt+B', true],
  ['panel.both', 'view', 'Ctrl+Shift+B', true],
  ['go.all', 'view', 'Ctrl+1', true],
  ['go.uncategorized', 'view', 'Ctrl+2', true],
  ['go.untagged', 'view', 'Ctrl+3', true],
  ['go.recent', 'view', 'Ctrl+4', true],
  ['go.random', 'view', 'Ctrl+5', true],
  ['go.tags', 'view', 'Ctrl+6', true],
  ['go.trash', 'view', 'Ctrl+7', true],
  ['layout.grid', 'view', 'Ctrl+Shift+1', true],
  ['layout.justified', 'view', 'Ctrl+Shift+2', true],
  ['layout.masonry', 'view', 'Ctrl+Shift+3', true],
  ['layout.list', 'view', 'Ctrl+Shift+4', true],
  ['layout.spectrum', 'view', 'Ctrl+Shift+5', true],
  ['look.gray', 'view', 'Alt+G', true],
  ['cards.larger', 'view', 'Ctrl+=', true],
  ['cards.smaller', 'view', 'Ctrl+-', true],
  ['scroll.top', 'view', 'Home', false],
  ['scroll.bottom', 'view', 'End', false],
  ['nav.back', 'view', 'Alt+ArrowLeft', false],
  ['nav.forward', 'view', 'Alt+ArrowRight', false],
  ['search.focus', 'find', 'Ctrl+F', true],
  ['filters.toggle', 'find', 'Ctrl+Shift+L', true],
  ['filters.clear', 'find', 'Ctrl+Shift+Backspace', true],
  ['sidebar.filter', 'find', 'Ctrl+Alt+F', true],
  ['item.toFolders', 'items', 'Ctrl+Shift+D', true],
  ['item.open', 'items', 'Space', false],
  ['item.actions', 'items', 'G', false],
  ['thumb.fromFile', 'items', 'Ctrl+Shift+T', false],
  ['thumb.fromClipboard', 'items', '', false],
  ['folder.defaultTags', 'items', '', false],
  ['viewer.rotate', 'player', 'R', false],
  ['viewer.flip', 'player', 'H', false],
  ['viewer.fileBack', 'player', 'PageUp', false],
  ['viewer.fileAhead', 'player', 'PageDown', false],
  ['viewer.zoomReset', 'player', 'Ctrl+0', false],
  ['text.mode', 'player', 'M', false],
  ['text.wrap', 'player', 'Alt+Z', false],
  ['text.lines', 'player', 'L', false],
  ['player.copyFrame', 'player', 'Alt+C', false],
  ['player.saveFrame', 'player', 'Alt+S', false],
  ['player.louder', 'player', 'Ctrl+ArrowUp', false],
  ['player.quieter', 'player', 'Ctrl+ArrowDown', false],
  ['player.seekAhead', 'player', 'Ctrl+ArrowRight', false],
  ['player.seekBack', 'player', 'Ctrl+ArrowLeft', false],
  ['player.faster', 'player', ']', false],
  ['player.slower', 'player', '[', false],
  ['player.frameBack', 'player', ',', false],
  ['player.frameAhead', 'player', '.', false],
  ['player.tenBack', 'player', 'Alt+,', false],
  ['player.tenAhead', 'player', 'Alt+.', false],
  ['item.openExternal', 'items', 'Shift+Alt+O', false],
  ['item.reveal', 'items', 'Shift+Alt+R', false],
  ['item.newWindow', 'items', 'Shift+Alt+W', false],
  ['item.toLastFolder', 'items', 'Alt+D', false],
  ['item.duplicate', 'items', 'Ctrl+D', false],
  ['item.copy', 'items', 'Ctrl+C', false],
  ['tags.copy', 'items', 'Ctrl+Shift+C', false],
  ['tags.paste', 'items', 'Ctrl+Shift+V', false],
  ['thumb.refresh', 'items', '', false],
  ['item.export', 'items', 'Ctrl+Shift+S', false],
  ['item.rename', 'items', 'F2', false],
  ['item.copyPath', 'items', 'Shift+Alt+C', false],
  ['item.selectAll', 'items', 'Ctrl+A', false],
  ['item.paste', 'items', 'Ctrl+V', false],
  ['item.trash', 'items', 'Delete', false],
  ['tags.add', 'items', 'Ctrl+K', false],
  ['tags.pick', 'items', 'T', false],
  ['folders.pick', 'items', 'F', false],
  ['rating.0', 'items', '0', false],
  ['rating.1', 'items', '1', false],
  ['rating.2', 'items', '2', false],
  ['rating.3', 'items', '3', false],
  ['rating.4', 'items', '4', false],
  ['rating.5', 'items', '5', false],
  ['rating.favorite', 'items', '', false],
  ['rating.explicit', 'items', '', false],
  ['folders.openAll', 'view', '/', false],
  ['folder.new', 'file', 'Ctrl+Shift+N', true],
  ['folder.newInside', 'file', 'Shift+Alt+N', true],
  ['smart.new', 'file', '', true],
  ['links.add', 'file', '', true],
  ['library.choose', 'file', 'Ctrl+Shift+O', true],
  ['app.settings', 'app', 'Ctrl+,', true],
  ['app.quit', 'app', 'Ctrl+Q', true],
]

// a Mac's delete key is Backspace, and Finder moves to the Trash with Command+Delete
const MAC_KEYS = { 'item.trash': 'Ctrl+Backspace' }

export const DEFAULT_KEYS = { ...Object.fromEntries(ACTIONS.map((a) => [a[0], a[2]])), ...(navigator.userAgent.includes('Mac') ? MAC_KEYS : {}) }

/** What a key passed on by a plugin's page may do: views, navigation, the viewer and the player, nothing that changes data or reaches other programs. */
export const FROM_PAGE = new Set(['panel.left', 'panel.right', 'panel.both', 'go.all', 'go.uncategorized', 'go.untagged', 'go.recent', 'go.random', 'go.tags', 'go.trash', 'layout.grid', 'layout.justified', 'layout.masonry', 'layout.list', 'layout.spectrum', 'look.gray', 'cards.larger', 'cards.smaller', 'scroll.top', 'scroll.bottom', 'nav.back', 'nav.forward', 'search.focus', 'filters.toggle', 'filters.clear', 'sidebar.filter', 'item.open', 'viewer.rotate', 'viewer.flip', 'viewer.fileBack', 'viewer.fileAhead', 'viewer.zoomReset', 'text.mode', 'text.wrap', 'text.lines', 'player.louder', 'player.quieter', 'player.seekAhead', 'player.seekBack', 'player.faster', 'player.slower', 'player.frameBack', 'player.frameAhead', 'player.tenBack', 'player.tenAhead', 'folders.openAll'])

const NAMES = { ' ': 'Space', '+': '=', Esc: 'Escape' }

/** Normalised combo for a keyboard event, e.g. "Ctrl+Shift+F". */
export function comboOf(e) {
  let k = NAMES[e.key] ?? e.key
  if (k.length === 1) k = k.toUpperCase()
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(k)) return ''
  // digits with Shift report symbols on some layouts: use the physical key
  if (e.code?.startsWith('Digit') && (e.shiftKey || e.altKey || e.ctrlKey)) k = e.code.slice(5)
  const mods = []
  if (e.ctrlKey || e.metaKey) mods.push('Ctrl')
  if (e.shiftKey && !(k.length === 1 && !/[A-Z0-9]/.test(k))) mods.push('Shift')
  if (e.altKey) mods.push('Alt')
  return [...mods, k].join('+')
}

/** A shortcut as menus show it, e.g. "Ctrl + Shift + J", keys under their English names (`keyText` names them as shown); empty when unassigned. */
export function shortcutLabel(keys, id) {
  const c = keys?.[id] ?? DEFAULT_KEYS[id]
  return c ? c.split('+').join(' + ') : ''
}

export function normalise(combo) {
  if (!combo) return ''
  const parts = combo.split('+').map((p) => p.trim()).filter(Boolean)
  const key = parts.pop() || ''
  const order = ['Ctrl', 'Shift', 'Alt']
  const mods = order.filter((m) => parts.some((p) => p.toLowerCase() === m.toLowerCase() || (m === 'Ctrl' && /^(cmd|cmdorctrl|command|meta)$/i.test(p))))
  return [...mods, key.length === 1 ? key.toUpperCase() : key].join('+')
}

/** The action bound to this event, if any. */
export function actionFor(e, keys) {
  const c = comboOf(e)
  if (!c) return null
  for (const [id] of ACTIONS) {
    if (normalise(keys[id] ?? DEFAULT_KEYS[id]) === c) return id
  }
  return null
}

export const actionName = (id) => t(`shortcuts.actions.${id}`)
export const groupName = (group) => t(`shortcuts.groups.${group}`)

export const typingAllowed = (id) => ACTIONS.find((a) => a[0] === id)?.[3]

const htmlText = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])

/** A tooltip naming its keys as small keycaps after the label, each combo (`Ctrl+Shift+L`) a run of them; plain text when there are none. */
export function keyTip(label, combos) {
  const list = combos.filter(Boolean)
  if (!list.length) return label
  const caps = list.map((c) => c.split('+').map((k) => `<kbd class="tip-key">${htmlText(keyName(k))}</kbd>`).join('')).join('<span class="tip-or">/</span>')
  return { value: `${htmlText(label)}<span class="tip-keys">${caps}</span>`, escape: false }
}

/** A button's tooltip with its shortcut as the keymap has it now, so it never names a key that does something else; `fixed` is a key that can't be changed, named first. */
export const withKey = (label, id, fixed = '') => keyTip(label, [fixed, S.ui.keys?.[id] ?? DEFAULT_KEYS[id]])
