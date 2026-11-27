// Curio's tools in windows of their own (Duplicates, Batch rename, Compare pictures): opening them for items, what they share in their windows, and the files dropped on them.
// A tool works with items as the API gives them (`id` is the item's lasting id); its thumbnails and previews need the grid's row of each, by that id.
import { reactive } from 'vue'
import { call } from '../platform/ipc.js'
import { S } from '../app/state.js'
import { run } from '../app/notify.js'
import { t } from '../shared/i18n.js'

export const TOOLS = ['duplicates', 'rename', 'compare']

/** A tool's window, opened or brought forward for these items (the window's own ids); `run` tells it more, as which entry opened it. */
export async function openTool(tool, rows = [], more = {}) {
  const items = rows.length ? await lastingIds(rows) : []
  return run('tool_window', { tool, title: t(`tools.titles.${tool}`), items, run: more })
}

async function lastingIds(rows) {
  const known = rows.map((id) => S.items.get(id)?.uid)
  if (known.every(Boolean)) return known
  return (await call('item.getMany', { ids: rows })).map((i) => i.id)
}

// ---------------------------------------------------------------- in a tool's window

/** The grid's rows of the items a tool shows, by their lasting id, for thumbnails and previews. */
export const rows = reactive(new Map())

const ROWS_CHUNK = 1000

export async function loadRows(ids) {
  const missing = [...new Set(ids)].filter((id) => !rows.has(id))
  for (let i = 0; i < missing.length; i += ROWS_CHUNK) for (const r of await call('grid_items_by_uid', { uids: missing.slice(i, i + ROWS_CHUNK) }).catch(() => [])) rows.set(r.uid, r)
}

/** Items through the API, a thousand at a time; `each` hears how many have come. */
export async function itemsOf(ids, each = () => {}) {
  const got = []
  for (let i = 0; i < ids.length; i += ROWS_CHUNK) {
    got.push(...(await call('item.getMany', { ids: ids.slice(i, i + ROWS_CHUNK) })))
    each(got.length)
  }
  return got
}

/** Work done in parts as a job, shown in Curio's status area where it can be cancelled between parts; answers how many were done. */
export async function asJob(title, list, size, part, each = () => {}) {
  const { job } = await call('job.start', { title, total: list.length })
  try {
    for (let i = 0; i < list.length; i += size) {
      const { cancelled } = await call('job.update', { job, done: i })
      if (cancelled) {
        await call('job.finish', { job }).catch(() => {})
        return i
      }
      await part(list.slice(i, i + size))
      each(Math.min(list.length, i + size))
    }
    await call('job.finish', { job, result: { done: list.length } })
    return list.length
  } catch (e) {
    await call('job.finish', { job, error: String(e?.message ?? e) }).catch(() => {})
    throw e
  }
}

/** Files dropped on a tool's window: those already items go to it as they are, the rest are copied in first. */
export async function dropFilesOnTool(paths, position) {
  const known = await call('items_at_paths', { paths }).catch(() => ({ ids: [], rest: paths }))
  const added = known.rest.length ? await run('item.add.fromPaths', { paths: known.rest, copy: true, onClash: 'keep', wait: true }) : null
  const had = known.ids.length ? (await call('item.getMany', { ids: known.ids }).catch(() => [])).map((i) => i.id) : []
  const items = [...had, ...(added?.ids || [])]
  if (items.length) S.toolWindow.drop = { items, ...at(position), seq: (S.toolWindow.drop?.seq || 0) + 1 }
}

/** Files held over a tool's window: where, until they leave or land. */
export function dragOverTool(position) {
  S.toolWindow.over = position ? at(position) : null
}

const at = (position) => {
  const k = window.devicePixelRatio || 1
  return Number.isFinite(position?.x) ? { x: position.x / k, y: position.y / k } : {}
}
