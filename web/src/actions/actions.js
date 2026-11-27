// Actions: named lists of steps run on the selected items with one click or key. The first five steps are
// the common ones first; the others extend them.
import { S } from '../app/state.js'
import { edit } from '../library/edits.js'
import { invalidateItems } from '../library/results.js'
import { loadDetail } from '../library/selection.js'
import { run, toast } from '../app/notify.js'
import { call, errorText } from '../platform/ipc.js'
import { t } from '../shared/i18n.js'

// each step's type and what it takes; its name is `actions.steps.<type>` in the catalogs
export const STEP_TYPES = [
  ['folders_add', 'folders'],
  ['folders_clear', null],
  ['tags_add', 'tags'],
  ['tags_clear', null],
  ['rating', 'rating'],
  ['tags_remove', 'tags'],
  ['folders_remove', 'folders'],
  ['note_set', 'text'],
  ['note_append', 'text'],
  ['url_set', 'text'],
  ['field_set', 'field'],
  ['rotate', null],
  ['thumb_refresh', null],
  ['colors_refresh', null],
  ['trash', null],
]

export const stepValue = (type) => STEP_TYPES.find((s) => s[0] === type)?.[1]
export const newStep = (type = 'tags_add') => ({ type, tags: [], folders: [], rating: 5, text: '', field: '', value: '' })

export async function loadActions() {
  const list = await call('library_json', { name: 'actions', set: null }).catch(() => [])
  S.actions = Array.isArray(list) ? list : []
}
export function saveActions() {
  return run('library_json', { name: 'actions', set: S.actions })
}

/** Turn one item by a quarter, the way Settings says. */
export async function rotateItem(id) {
  const it = S.items.get(id)
  // a read-only location keeps its files as they are, so only the view turns
  if (S.ui.imageRotation === 'file' && !it?.read_only) return call('item.rotateFile', { id })
  const degrees = ((it?.rotation || 0) + 90) % 360
  if (it) it.rotation = degrees
  S.itemsVer++
  return call('item.rotate', { ids: [id], degrees })
}

async function runStep(step, ids) {
  switch (step.type) {
    case 'tags_add':
      if (step.tags.length) await edit.addTags(ids, step.tags)
      return
    case 'tags_clear':
      await edit.clearTags(ids)
      return
    case 'tags_remove':
      for (const name of step.tags) {
        const tag = S.sidebar.tags.find((x) => x.name === name)
        if (tag) await edit.removeTag(ids, tag.id)
      }
      return
    case 'folders_add':
      for (const id of step.folders) await edit.folder('add', { id, ids })
      return
    case 'folders_remove':
      for (const id of step.folders) await edit.folder('unadd', { id, ids })
      return
    case 'folders_clear':
      await edit.folder('clear', { ids })
      return
    case 'rating':
      await edit.rating(ids, Number(step.rating))
      return
    case 'note_set':
      await edit.text(ids, 'annotation', step.text)
      return
    case 'note_append':
      {
        const brief = await call('assets_brief', { ids })
        const byNote = new Map()
        for (const id of ids) {
          const note = [brief[id]?.annotation, step.text].filter(Boolean).join('\n')
          byNote.set(note, [...(byNote.get(note) || []), id])
        }
        for (const [note, same] of byNote) await edit.text(same, 'annotation', note)
      }
      return
    case 'url_set':
      await edit.text(ids, 'url', step.text)
      return
    case 'field_set':
      if (step.field) await edit.field(ids, step.field, step.value)
      return
    case 'rotate':
      for (const id of ids) await rotateItem(id).catch((e) => toast(errorText(e), 'error'))
      return
    case 'thumb_refresh':
      invalidateItems(ids)
      await call('item.refreshThumbnail', { ids })
      return
    case 'colors_refresh':
      await call('item.refreshPalette', { ids })
      return
    case 'trash':
      edit.trash(ids, true)
      return
  }
  // a plugin's program (`where: step`), waited for so the next step sees what it did
  if (step.type.startsWith('plugin:')) {
    await call('plugin_op', { op: 'run', id: step.type.slice(7), enabled: null, granted: null, ids })
    invalidateItems(ids)
  }
}

export async function runUserAction(a) {
  const ids = [...S.selected]
  if (!ids.length) return toast(t('common.selectFirst'), 'error')
  try {
    for (const step of a.steps) await runStep(step, ids)
    toast(t('actions.done', { name: a.name, count: ids.length }))
    loadDetail()
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
