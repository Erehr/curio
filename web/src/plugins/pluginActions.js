// Plugin actions: what plugins add to the item menu, the toolbar's Plugins menu, the viewer and Curio's own actions (`actions` in plugin.json).
import { S } from '../app/state.js'
import { run } from '../app/notify.js'
import { ACTIONS, DEFAULT_KEYS, actionName, comboOf, normalise } from '../app/shortcuts.js'

/** The plugin actions shown in one place: item-menu, toolbar, viewer or step. */
export const pluginActions = (place) => (S.contrib.actions || []).filter((a) => a.where.includes(place))

const COUNTS = { one: (n) => n === 1, many: (n) => n > 1, some: (n) => n > 0, any: () => true }

/** Whether an action applies to these items: their number, and the kinds and extensions of those the window knows. */
export function actionFits(a, rows) {
  if (!(COUNTS[a.selection] || COUNTS.some)(rows.length)) return false
  if (!a.kinds.length && !a.exts.length) return true
  return rows.every((id) => {
    const it = S.items.get(id)
    return !it || ((!a.kinds.length || a.kinds.includes(it.kind)) && (!a.exts.length || a.exts.includes(String(it.ext).toLowerCase())))
  })
}

// pages are told items by their stable ids; one the window hasn't loaded goes by its number, which the API takes too
const stableIds = (rows) => rows.map((id) => S.items.get(id)?.uid ?? id)

/** Opens the action's page with the items, or runs its program on them and waits for it. */
export async function runPluginAction(a, rows) {
  if (a.opens) return openPluginPage(a.plugin, a.opens, { action: a.id, items: stableIds(rows) })
  return run('plugin_op', { op: 'run', id: `${a.plugin}::${a.id}`, enabled: null, granted: null, ids: rows })
}

/** A plugin's window, or its tab as `tab:<id>`, hearing `data` in plugin.run. */
export function openPluginPage(plugin, opens, data = {}) {
  if (!opens.startsWith('tab:')) return run('plugin_window', { plugin, window: opens, items: null, run: data })
  const key = `tab:${plugin}/${opens.slice(4)}`
  if (!(S.contrib.tabs || []).some((t) => `tab:${t.plugin}/${t.id}` === key)) throw new Error(`${plugin} has no tab ${opens.slice(4)}`)
  S.viewer = null
  S.tabRun = { key, data: { tab: opens.slice(4), ...data }, seq: (S.tabRun?.seq || 0) + 1 }
  S.view = key
}

/** A Window plugin's window, for the selected items. */
export function openPluginWindow(plugin, id) {
  return openPluginPage(plugin, id, { items: stableIds(S.selected) })
}

// ---------------------------------------------------------------- shortcuts
// Every plugin action can have a shortcut, kept with Curio's own in S.ui.keys under this id; '' there is one taken away.
export const actionKeyId = (a) => `plugin:${a.plugin}::${a.id}`

/** Whether a key is one of Curio's own shortcuts, as bound now. */
const curioHas = (combo, keys) => ACTIONS.some(([id]) => normalise(keys[id] ?? DEFAULT_KEYS[id]) === combo)

/** The key an action comes with; one that is already Curio's comes without one. */
export function actionDefaultKey(a, keys = S.ui.keys || {}) {
  const k = normalise(a.key || '')
  return k && !curioHas(k, keys) ? k : ''
}

/** An action's key now: the person's, or the one it comes with. */
export const actionKey = (a, keys = S.ui.keys || {}) => normalise(keys[actionKeyId(a)] ?? actionDefaultKey(a, keys))

/** The actions that have this key now, but `except` (a shortcut id: Curio's action, `plugin:…` or `action:<id>` for one of yours), as `{ id, label }`. */
export function keyHolders(combo, except, { keys = S.ui.keys || {}, plugins = S.contrib.actions || [], mine = S.actions || [], viewer = S.contrib.viewerKeys || [] } = {}) {
  const c = normalise(combo || '')
  if (!c) return []
  // a viewer page's keys work only while it shows, over every other
  const vk = viewer.find((k) => viewerKeyId(k) === except)
  if (vk) return viewer.filter((k) => k !== vk && k.plugin === vk.plugin && viewerKey(k, keys) === c).map((k) => ({ id: viewerKeyId(k), label: k.title }))
  const bindable = plugins.filter((a) => a.where.some((w) => w !== 'step'))
  const out = []
  for (const [id] of ACTIONS) if (id !== except && normalise(keys[id] ?? DEFAULT_KEYS[id]) === c) out.push({ id, label: actionName(id) })
  for (const a of bindable) if (actionKeyId(a) !== except && actionKey(a, keys) === c) out.push({ id: actionKeyId(a), label: a.title.replace(/…$/, '') })
  for (const a of mine) if (`action:${a.id}` !== except && a.shortcut && normalise(a.shortcut) === c) out.push({ id: `action:${a.id}`, label: a.name })
  return out
}

/** The plugin action this key runs on the selected items, if one applies to them. */
export function pluginActionFor(e, rows) {
  const c = comboOf(e)
  if (!c) return null
  return (S.contrib.actions || []).find((a) => !a.disabled && a.where.some((w) => w !== 'step') && actionKey(a) === c && actionFits(a, rows)) || null
}

// ---------------------------------------------------------------- viewer keys
// A viewer page's keys (`viewer_keys` in plugin.json) come to it as `plugin.tool`; kept in S.ui.keys as the actions' are.
export const viewerKeyId = (k) => `plugin:${k.plugin}::viewer:${k.id}`

export const viewerKey = (k, keys = S.ui.keys || {}) => normalise(keys[viewerKeyId(k)] ?? (k.key || ''))

/** The key of the viewer page of `plugin` this press is, among those the page last said work now (`keys` in viewer.setTools; all of them unless it did). */
export function viewerKeyFor(e, plugin) {
  const c = comboOf(e)
  if (!c || !plugin) return null
  const now = S.viewerTools?.plugin === plugin ? S.viewerTools.keys : null
  return (S.contrib.viewerKeys || []).find((k) => k.plugin === plugin && viewerKey(k) === c && (!now || now.includes(k.id))) || null
}

/** A plugin's keys as they are now, for its pages: `{ actions: { id: 'F2' }, viewer: { id: 'T' } }`, the ones without a key left out. */
export function pluginKeys(plugin, keys = S.ui.keys || {}) {
  const of = (list, keyOf) => Object.fromEntries(list.filter((x) => x.plugin === plugin).map((x) => [x.id, keyOf(x, keys)]).filter(([, k]) => k))
  return { actions: of(S.contrib.actions || [], actionKey), viewer: of(S.contrib.viewerKeys || [], viewerKey) }
}
