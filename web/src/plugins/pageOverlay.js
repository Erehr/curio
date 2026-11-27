// What a plugin page opens over the window, outside its frame: a select's list (ui.choose) and a popover showing another of its plugin's pages (ui.popover). plugins/PageOverlay.vue draws them; plugins/bridge.js asks for them.
import { reactive } from 'vue'

export const overlay = reactive({ choose: null, popover: null })

const fail = (code, message) => Object.assign(new Error(message), { code })
const num = (x) => (Number.isFinite(Number(x)) ? Number(x) : 0)
const MAX_OPTIONS = 1000

/** The page's box for what it opens, from the page's own coordinates into the window's. */
export function fieldRect(iframe, at) {
  const f = iframe.getBoundingClientRect()
  const a = at && typeof at === 'object' ? at : {}
  const w = Math.max(0, num(a.w))
  const h = Math.max(0, num(a.h))
  return { left: f.left + num(a.x), top: f.top + num(a.y), right: f.left + num(a.x) + w, bottom: f.top + num(a.y) + h, width: w, height: h }
}

const plainValue = (v) => (['string', 'number', 'boolean'].includes(typeof v) ? v : null)

/** A list of options under the field; answers the one chosen, or nothing when it was closed. */
export function choose(rect, args) {
  if (!Array.isArray(args.options) || !args.options.length) throw fail('bad_request', 'ui.choose needs `options`')
  const options = args.options.slice(0, MAX_OPTIONS).map((o) => (o && typeof o === 'object' ? { value: plainValue(o.value), label: String(o.label ?? o.value ?? '').slice(0, 200), icon: typeof o.icon === 'string' ? o.icon.slice(0, 60) : null, hint: typeof o.hint === 'string' ? o.hint.slice(0, 120) : null, heading: !!o.heading, disabled: !!o.disabled } : { value: plainValue(o), label: String(o).slice(0, 200), icon: null, hint: null, heading: false, disabled: false }))
  overlay.choose?.resolve({ chosen: false })
  return new Promise((resolve) => (overlay.choose = { rect, options, value: plainValue(args.value), search: !!args.search, resolve }))
}

/** One of the plugin's own pages in a popover under the field; answers what that page closed it with. */
export function popover(rect, plugin, args) {
  const page = String(args.page ?? '')
  if (!page || /^[a-z][a-z0-9+.-]*:|^\/|\.\.(\/|$)/i.test(page)) throw fail('bad_request', 'ui.popover needs `page`, a path in the plugin\'s folder')
  overlay.popover?.resolve({ result: null })
  const width = Math.max(160, Math.min(640, num(args.width) || 300))
  return new Promise((resolve) => (overlay.popover = { rect, plugin, page: page.slice(0, 300), data: args.data ?? null, width, height: Math.max(0, num(args.height)), resolve }))
}

/** The popover's page closed it: with what it gives. */
export function closePopover(result) {
  const p = overlay.popover
  if (!p) return
  overlay.popover = null
  p.resolve({ result: result ?? null })
}
