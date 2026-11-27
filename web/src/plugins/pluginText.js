// The text plugins bring through Rust (names, descriptions, field labels, widget and action titles), in the window's language: the text Rust itself defines for Curio's built-in entries is in app.json under `plugins.builtin`.
import { englishText, t } from '../shared/i18n.js'

// a plugin's id with its dots as dashes, as `plugins.builtin` keys it
const slug = (owner) => String(owner).replace(/\./g, '-')

/** `raw`, the English a plugin gave for the text at `path` (`name`, `widgets.<id>`, `fields.<key>.label`), in the window's language; text the user changed, or another plugin's, stays as it is. */
export function ownText(owner, path, raw) {
  if (!owner || typeof raw !== 'string' || !raw) return raw
  const s = slug(owner)
  if (englishText(`plugins.builtin.${s}.${path}`) === raw) return t(`plugins.builtin.${s}.${path}`)
  return raw
}

const titled = (list, path, field = 'title') => (list || []).map((x) => ({ ...x, [field]: ownText(x.plugin, `${path}.${path === 'sorts' ? x.field : x.id}`, x[field]) }))

/** The library's fields, as `sidebar` gives them, with their labels and descriptions in the window's language. */
export const localizeFields = (fields) => (fields || []).map((f) => ({ ...f, label: ownText(f.owner, `fields.${f.key}.label`, f.label), description: ownText(f.owner, `fields.${f.key}.description`, f.description) }))

/** What plugins add, as `contributions` gives it, in the window's language. */
export function localizeContrib(c) {
  if (!c) return c
  return {
    ...c,
    widgets: titled(c.widgets, 'widgets'),
    tabs: titled(c.tabs, 'tabs'),
    windows: titled(c.windows, 'windows'),
    viewers: (c.viewers || []).map((v) => (v.screen ? { ...v, screen: { ...v.screen, title: ownText(v.plugin, `screens.${v.screen.id}`, v.screen.title) } } : v)),
    names: Object.fromEntries(Object.entries(c.names || {}).map(([id, name]) => [id, ownText(id, 'name', name)])),
    viewerTools: titled(c.viewerTools, 'viewerTools'),
    viewerKeys: titled(c.viewerKeys, 'viewerKeys'),
    actions: titled(c.actions, 'actions'),
    filters: titled(c.filters, 'filters', 'label'),
    sorts: titled(c.sorts, 'sorts', 'label'),
    columns: titled(c.columns, 'columns', 'label'),
  }
}

/** The plugins, as `plugin_list` gives them, in the window's language. */
export function localizePlugins(list) {
  const names = Object.fromEntries((list || []).map((p) => [p.name, ownText(p.id, 'name', p.name)]))
  return (list || []).map((p) => ({
    ...p,
    name: names[p.name],
    description: ownText(p.id, 'description', p.description),
    windows: (p.windows || []).map((w) => ({ ...w, title: ownText(p.id, `windows.${w.id}`, w.title) })),
    tabs: (p.tabs || []).map((x) => ({ ...x, title: ownText(p.id, `tabs.${x.id}`, x.title) })),
    overriddenBy: (p.overriddenBy || []).map((o) => ({ ...o, plugin: names[o.plugin] || o.plugin })),
  }))
}
