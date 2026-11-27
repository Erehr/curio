// The host side of plugin pages: each page runs in a sandboxed frame, and its calls reach the API as its plugin's through this page (src/core/api/client/curio.js has the frame's side).
// A frame is known by its window object, which the page inside can't change, so it can't speak for another plugin or for the window.
import { effectScope, watch } from 'vue'
import { call, emit, invoke, on } from '../platform/ipc.js'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S, themeShown } from '../app/state.js'
import { run } from '../app/notify.js'
import { pointer } from '../app/windowRequests.js'
import { pageOrigin } from './pageLabel.js'
import { choose, closePopover, fieldRect, popover } from './pageOverlay.js'
import { pluginKeys } from './pluginActions.js'
import { language } from '../shared/i18n.js'

/** Events the page showing a frame tells it itself; the rest come from the API. */
const LOCAL = ['plugin.run', 'plugin.show', 'plugin.hide', 'plugin.beforeExit', 'plugin.item', 'plugin.selection', 'plugin.drop', 'plugin.dragOver', 'plugin.tool', 'plugin.thumbnail', 'plugin.extract', 'plugin.settingsChanged', 'plugin.keys']
/** Told once more to a page that starts listening late: what it was opened for and what it shows. */
const STICKY = ['plugin.run', 'plugin.item', 'plugin.selection', 'plugin.keys']
/** Kept until the page listens, so none is lost while it loads. */
const QUEUED = ['plugin.thumbnail', 'plugin.extract']
/** Operations the page showing the frame answers itself. */
const PAGE_OPS = { 'viewer.setTools': setTools, 'plugin.close': closeWindow, 'ui.choose': chooseOption, 'ui.popover': openPopover }

// a select's list and a popover open over the window, which a frame can't draw outside itself; under the page's field (`at`, in the page's coordinates)
async function chooseOption(f, args) {
  const r = await choose(fieldRect(f.iframe, args.at), args)
  // the page keeps the keys, as it had them before its list opened
  f.iframe.focus()
  return r
}
function openPopover(f, args) {
  return popover(fieldRect(f.iframe, args.at), f.plugin, args)
}

const frames = new Map()
let seq = 0
let started = false
const label = getCurrentWindow().label

const post = (f, m) => f.win?.postMessage({ curio: 1, ...m }, f.origin)
// a frame speaks for its plugin only while it shows the plugin's own page: one navigated elsewhere is neither heard nor told; WebKitGTK's origin for plugin:// pages isn't relied on
const originOf = (plugin) => (navigator.userAgent.includes('Windows') ? pageOrigin(plugin, true) : '*')
const plain = (v) => JSON.parse(JSON.stringify(v ?? {}))

function tellTheme(f) {
  if (themeShown.theme) post(f, { theme: themeShown.theme, seeThrough: themeShown.seeThrough, inactive: !document.hasFocus(), role: f.role, overlayScroll: S.ui.overlayScrollbars !== false, lang: language(), wheel: S.ui.viewerWheel || 'navigate', rememberZoom: !!S.ui.rememberZoom })
}

const listens = (f, name) => [...f.local.values()].some((names) => names.includes(name) || names.includes('plugin.*'))

/** Tells a frame's own subscribers one of the events the page itself sends. */
function tellLocal(f, name, data = {}) {
  if (STICKY.includes(name)) f.sticky[name] = data
  if (QUEUED.includes(name) && !listens(f, name)) return f.queue.push([name, data])
  for (const [sub, names] of f.local) if (names.includes(name) || names.includes('plugin.*')) post(f, { sub, event: { id: null, name, library: null, at: Date.now(), data } })
}

const str = (x, n = 80) => String(x ?? '').slice(0, n)
const num = (x, d) => (Number.isFinite(Number(x)) ? Number(x) : d)
const plainValue = (v) => (['string', 'number', 'boolean'].includes(typeof v) ? v : null)
const colour = (c) => (typeof c === 'string' && /^[#\w(),.%\s-]{1,40}$/.test(c) ? c : null)
const list = (x, n) => (Array.isArray(x) ? x.slice(0, n) : [])

/** One control of a toolbar button's panel, as Curio draws it. */
function control(c) {
  const type = str(c?.type, 12)
  const base = { type, id: str(c.id), label: str(c.label, 60) }
  if (type === 'section') return { type, label: base.label }
  if (type === 'separator') return { type }
  if (type === 'slider') {
    const min = num(c.min, 0)
    const max = Math.max(min, num(c.max, 100))
    return { ...base, min, max, step: num(c.step, 1) || 1, value: Math.min(max, Math.max(min, num(c.value, min))), unit: str(c.unit, 8) }
  }
  if (type === 'toggle') return { ...base, value: !!c.value }
  if (type === 'choice' || type === 'select' || type === 'swatches') {
    const options = list(c.options, type === 'select' ? 500 : 12).map((o) => ({ value: plainValue(o?.value), label: str(o?.label ?? o?.value, 60), icon: o?.icon ? str(o.icon, 40) : null, background: colour(o?.background), foreground: colour(o?.foreground) }))
    return { ...base, value: plainValue(c.value), options }
  }
  return null
}

/** A panel's controls, in tabs when it names them: `[{ tab, controls }]`, or the controls alone. */
function panel(p) {
  if (!Array.isArray(p) || !p.length) return null
  const tabs = p.some((t) => t && t.tab != null) ? list(p, 6).map((t) => ({ label: str(t.tab, 30), icon: t.icon ? str(t.icon, 40) : null, controls: list(t.controls, 40).map(control).filter(Boolean) })) : [{ label: '', icon: null, controls: list(p, 40).map(control).filter(Boolean) }]
  return tabs.filter((t) => t.controls.length)
}

function setTools(f, args) {
  if (f.role !== 'viewer') throw Object.assign(new Error('Only a viewer page has buttons in the toolbar'), { code: 'not_allowed' })
  const buttons = list(args.buttons, 12).map((b) => ({ id: str(b.id), label: str(b.label ?? b.id, 200), icon: b.icon ? str(b.icon, 40) : null, on: !!b.on, panel: panel(b.panel) }))
  const sl = args.slider
  const min = num(sl?.min, 0)
  const max = Math.max(min, num(sl?.max, 1))
  const slider = sl && sl.id != null ? { id: str(sl.id), label: str(sl.label, 120), min, max, step: num(sl.step, 1) || 1, value: Math.min(max, Math.max(min, num(sl.value, min))), dir: sl.dir === 'rtl' ? 'rtl' : 'ltr' } : null
  const labels = list(args.labels, 12).map((l) => ({ id: str(l.id), text: str(l.text, 200) })).filter((l) => l.text)
  const keys = Array.isArray(args.keys) ? list(args.keys, 64).map((k) => str(k)) : null
  S.viewerTools = buttons.length || labels.length || slider || keys ? { frame: f.n, plugin: f.plugin, labels, buttons, slider, keys } : null
  return { success: true }
}

function closeWindow(f, args) {
  if (f.role === 'popover') {
    closePopover(args.result === undefined ? null : plain(args.result))
    return { success: true }
  }
  if (f.role !== 'window' || !S.pluginWindow) throw Object.assign(new Error('Only a page in a window of its own closes it'), { code: 'not_allowed' })
  getCurrentWindow().close()
  return { success: true }
}

async function answer(f, m) {
  if (m.op && PAGE_OPS[m.op]) {
    try {
      return { ok: await PAGE_OPS[m.op](f, m.args ?? {}) }
    } catch (e) {
      return { error: { code: e.code || 'failed', message: e.message } }
    }
  }
  if (m.op) {
    const r = await invoke('plugin_api', { plugin: f.plugin, op: m.op, args: m.args ?? {} })
    // a file answer comes apart from `ok`, which holds whatever the plugin stored
    if (r.file) {
      const bytes = await invoke('plugin_file', { key: r.file.key })
      return { ok: new Blob([bytes], { type: r.file.type }) }
    }
    // the plugin's other pages, in every window: its settings page is in Settings' own
    if (m.op === 'plugin.settings.set' && r.ok) emit('plugin-settings', { plugin: f.plugin, values: plain(r.ok), from: `${label}:${f.n}` }).catch(() => {})
    return r
  }
  if (m.subscribe) {
    const asked = Array.isArray(m.subscribe.events) ? m.subscribe.events : []
    const local = asked.filter((n) => LOCAL.includes(n) || n === 'plugin.*')
    const rest = asked.filter((n) => !local.includes(n))
    const sub = m.id
    if (!asked.length || rest.length) {
      const key = `${label}:${f.n}:${sub}`
      const r = await invoke('plugin_subscribe', { plugin: f.plugin, sub: key, filter: { events: rest, ids: m.subscribe.ids || [] } })
      if (r.error) return r
      f.remote.set(key, sub)
    }
    if (local.length) f.local.set(sub, local)
    // what the page was opened for and what it shows, once, when it first listens for them; and what waited for it
    setTimeout(() => {
      for (const name of STICKY) if (f.sticky[name] && !f.told.has(name) && listens(f, name)) {
        f.told.add(name)
        tellLocal(f, name, f.sticky[name])
      }
      const waiting = f.queue.filter(([name]) => listens(f, name))
      f.queue = f.queue.filter(([name]) => !listens(f, name))
      for (const [name, data] of waiting) tellLocal(f, name, data)
    })
    return { ok: { sub } }
  }
  if (m.unsubscribe != null) {
    f.local.delete(m.unsubscribe)
    for (const [key, sub] of f.remote) {
      if (sub !== m.unsubscribe) continue
      f.remote.delete(key)
      invoke('plugin_unsubscribe', { sub: key }).catch(() => {})
    }
    return { ok: {} }
  }
  if (m.thumbnailDone || m.extractDone) {
    f.onAnswer?.(m.thumbnailDone || m.extractDone)
    return { ok: {} }
  }
  if (m.hello) {
    // a page starting in the frame follows nothing yet, and hears what it shows again
    endSubs(f)
    f.told.clear()
    if (S.viewerTools?.frame === f.n) S.viewerTools = null
    tellTheme(f)
    return { ok: { plugin: f.plugin } }
  }
  return { error: { code: 'bad_request', message: 'Curio doesn\'t understand this message' } }
}

/** A key the page didn't use, pressed as if in this window, so the viewer and Curio's shortcuts still answer.
 * A page can send any key at any time, so only the focused frame is heard, and the event is marked for `onKey` to answer only shortcuts that change no data. */
function replayKey(f, k) {
  if (!k || typeof k.key !== 'string' || document.activeElement !== f.iframe) return
  const e = new KeyboardEvent('keydown', { key: k.key, code: String(k.code || ''), ctrlKey: !!k.ctrl, shiftKey: !!k.shift, altKey: !!k.alt, metaKey: !!k.meta, repeat: !!k.repeat, bubbles: true, cancelable: true })
  Object.defineProperty(e, 'fromPage', { value: true })
  window.dispatchEvent(e)
}

function start() {
  if (started) return
  started = true
  // what an earlier page of this window followed ended with it
  invoke('plugin_unsubscribe', { sub: null }).catch(() => {})
  window.addEventListener('message', async (e) => {
    const f = [...frames.values()].find((x) => x.win === e.source)
    const m = e.data
    if (!f || !m || m.curio !== 1 || (f.origin !== '*' && e.origin !== f.origin)) return
    if (m.key) return replayKey(f, m.key)
    // a press in a page, which the window's own listeners never hear: its menus and popovers close as for a press anywhere else
    if (m.down) return window.dispatchEvent(new CustomEvent('curio-frame-down', { detail: f.iframe }))
    // how tall a widget's page is, for its frame to fit it
    if (m.height != null) return ['widget', 'popover', 'settings'].includes(f.role) && f.iframe.dispatchEvent(new CustomEvent('curio-height', { detail: Math.max(0, Math.min(4000, Number(m.height) || 0)) }))
    if (m.title != null) return f.role === 'widget' && f.iframe.dispatchEvent(new CustomEvent('curio-title', { detail: String(m.title) }))
    if (m.id == null) return
    // a menu asked for from a toolbar button opens at the button, not where the pointer last was in the page
    if (f.toolAt && performance.now() < f.toolAt.until) {
      pointer.x = f.toolAt.x
      pointer.y = f.toolAt.y
      if (m.op === 'ui.menu') f.toolAt = null
    } else if (m.pointer && Number.isFinite(m.pointer.x) && Number.isFinite(m.pointer.y)) {
      const at = f.iframe.getBoundingClientRect()
      pointer.x = at.left + m.pointer.x
      pointer.y = at.top + m.pointer.y
    }
    let r
    try {
      r = await answer(f, m)
    } catch (err) {
      r = { error: { code: 'failed', message: String(err?.message ?? err) } }
    }
    post(f, { id: m.id, ...r })
  })
  on('plugin-settings', ({ plugin, values, from }) => {
    for (const f of frames.values()) if (f.plugin === plugin && `${label}:${f.n}` !== from) tellLocal(f, 'plugin.settingsChanged', values)
  })
  on('plugin-event', ({ sub, to, event }) => {
    if (to !== label) return
    for (const f of frames.values()) if (f.remote.has(sub)) post(f, { sub: f.remote.get(sub), event })
  })
  // started by the first frame's component, whose unmounting would stop a watch of its own: this one serves every frame from then on
  effectScope(true).run(() => watch(() => [S.themeVer, S.ui.overlayScrollbars, S.ui.viewerWheel, S.ui.rememberZoom, language()], () => frames.forEach(tellTheme)))
  // an inactive window's panels turn solid, the pages' with them
  for (const e of ['focus', 'blur']) window.addEventListener(e, () => themeShown.seeThrough && frames.forEach(tellTheme))
  document.addEventListener('visibilitychange', () => frames.forEach((f) => f.visible && tellLocal(f, document.hidden ? 'plugin.hide' : 'plugin.show')))
  window.addEventListener('pagehide', () => frames.forEach((f) => tellLocal(f, 'plugin.beforeExit')))
}

function endSubs(f) {
  for (const key of f.remote.keys()) invoke('plugin_unsubscribe', { sub: key }).catch(() => {})
  f.remote.clear()
  f.local.clear()
}

/**
 * Lets the page in `iframe` speak for `plugin`. `role` is window, tab, widget, viewer, thumbnail or extractor; `onAnswer` hears a thumbnail's or an extractor's answer; `run` is what it was opened for.
 * Answers a handle: `tell(name, data)` sends it one of the page's own events, `item(data)` what a viewer shows now, `selection(data)` what a widget's window has selected, `run(data)` what it was opened for again, `visible(on)` whether it can be seen, `detach()` ends it.
 */
export function attachFrame(iframe, plugin, { role = 'window', run = {}, onAnswer = null } = {}) {
  start()
  const f = { n: ++seq, plugin, origin: originOf(plugin), role, iframe, win: iframe.contentWindow, local: new Map(), remote: new Map(), sticky: { 'plugin.run': plain(run), 'plugin.keys': pluginKeys(plugin) }, told: new Set(), queue: [], visible: true, onAnswer }
  frames.set(f.n, f)
  iframe.dataset.drop = `frame:${f.n}`
  // a page without curio.js still gets the theme's variables
  const loaded = () => tellTheme(f)
  iframe.addEventListener('load', loaded)
  return {
    n: f.n,
    tell: (name, data) => tellLocal(f, name, plain(data)),
    item: (data) => {
      if (listens(f, 'plugin.item')) f.told.add('plugin.item')
      tellLocal(f, 'plugin.item', plain(data))
    },
    run: (data) => {
      if (listens(f, 'plugin.run')) f.told.add('plugin.run')
      tellLocal(f, 'plugin.run', plain(data))
    },
    selection: (data) => {
      if (listens(f, 'plugin.selection')) f.told.add('plugin.selection')
      tellLocal(f, 'plugin.selection', plain(data))
    },
    visible: (on) => {
      if (on === f.visible) return
      f.visible = on
      tellLocal(f, on ? 'plugin.show' : 'plugin.hide')
    },
    detach: () => {
      iframe.removeEventListener('load', loaded)
      tellLocal(f, 'plugin.beforeExit')
      endSubs(f)
      if (S.viewerTools?.frame === f.n) S.viewerTools = null
      frames.delete(f.n)
    },
  }
}

/** Items dragged from the grid onto a frame: the page hears their ids as `plugin.drop`. */
/** Where in the page a drop at window point `x`, `y` landed, so a page can tell its parts apart. */
const inFrame = (f, x, y) => {
  if (!Number.isFinite(x) || !Number.isFinite(y)) return {}
  const r = f.iframe.getBoundingClientRect()
  return { x: Math.round(x - r.left), y: Math.round(y - r.top) }
}

export async function dropOnFrame(n, rows, at = {}) {
  const f = frames.get(n)
  if (!f || !rows?.length) return
  const items = await call('item.getMany', { ids: rows.slice(0, 1000) }).catch(() => [])
  tellLocal(f, 'plugin.drop', { items: items.map((i) => i.id), ...inFrame(f, at.x, at.y) })
}

/** Files dropped on a plugin's own window. Those already in the library, as items dragged out of its other windows are, go to the page as they are; the rest are added without asking about names, as the window has no folder to keep apart, and copied in. The page hears `plugin.drop`. */
export async function dropFilesOnWindow(paths, position) {
  const f = [...frames.values()].find((x) => x.role === 'window')
  if (!f || !paths?.length) return
  const known = await call('items_at_paths', { paths }).catch(() => ({ ids: [], rest: paths }))
  const added = known.rest.length ? await run('item.add.fromPaths', { paths: known.rest, copy: true, onClash: 'keep', wait: true }) : null
  const had = known.ids.length ? (await call('item.getMany', { ids: known.ids }).catch(() => [])).map((i) => i.id) : []
  const items = [...had, ...(added?.ids || [])]
  const k = window.devicePixelRatio || 1
  if (items.length) tellLocal(f, 'plugin.drop', { items, ...inFrame(f, position?.x / k, position?.y / k) })
}

/** Files held over a plugin's own window: the page hears where, as `plugin.dragOver` with `x` and `y`, and with neither once they leave or land. */
export function dragOverWindow(position) {
  const f = [...frames.values()].find((x) => x.role === 'window')
  if (!f) return
  const k = window.devicePixelRatio || 1
  tellLocal(f, 'plugin.dragOver', position ? inFrame(f, position.x / k, position.y / k) : {})
}

// a page shows its keys as they are bound now
watch(() => JSON.stringify([S.ui.keys, S.contrib.actions, S.contrib.viewerKeys]), () => {
  for (const f of frames.values()) {
    const keys = pluginKeys(f.plugin)
    if (JSON.stringify(keys) !== JSON.stringify(f.sticky['plugin.keys'])) tellLocal(f, 'plugin.keys', keys)
  }
})

/** The plugin whose page the viewer shows, or whose entry the window's own viewer shows (Fonts), if one does. */
export const viewerPlugin = () => [...frames.values()].find((f) => f.role === 'viewer')?.plugin ?? (S.viewerTools?.onKey ? S.viewerTools.plugin : null)

/** One of the viewer page's keys, told as a click of the tool of its id. */
export function viewerKeyPressed(id) {
  // a viewer of the window's own (formats/FontView.vue) takes its keys itself
  if (S.viewerTools?.onKey) return S.viewerTools.onKey(id)
  const f = [...frames.values()].find((x) => x.role === 'viewer')
  if (f) tellLocal(f, 'plugin.tool', { id, key: true })
}

/** A click on one of a viewer page's toolbar buttons; a right-click (or a long press) asks for its menu. */
export function toolClicked(id, e = null) {
  if (S.viewerTools?.onTool) return S.viewerTools.onTool(id, e)
  const f = frames.get(S.viewerTools?.frame)
  if (!f) return
  if (e) {
    const r = e.currentTarget.getBoundingClientRect()
    f.toolAt = { x: r.left, y: r.bottom + 4, until: performance.now() + 2000 }
  }
  tellLocal(f, 'plugin.tool', e ? { id, menu: true } : { id })
}

/** A value set with one of a viewer page's toolbar controls: its slider, or a control in a button's panel. */
export function toolChanged(id, value) {
  if (S.viewerTools?.onChange) return S.viewerTools.onChange(id, value)
  const f = frames.get(S.viewerTools?.frame)
  if (f) tellLocal(f, 'plugin.tool', { id, value })
}

/** Where a plugin's page is served, under the plugin's own origin. */
export function pageUrl(plugin, page) {
  const base = pageOrigin(plugin, navigator.userAgent.includes('Windows'))
  const s = String(page)
  const at = s.search(/[?#]/)
  const [path, query] = at < 0 ? [s, ''] : [s.slice(0, at), s.slice(at)]
  return `${base}/${encodeURIComponent(plugin)}/${path.split('/').map(encodeURIComponent).join('/')}${query}`
}
