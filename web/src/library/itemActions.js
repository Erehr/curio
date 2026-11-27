// What can be done with the selected items both from their menu and with a key: tags copied and pasted, the files copied, duplicated or exported, a thumbnail chosen or made again, a window of their own.
import { S } from '../app/state.js'
import { t } from '../shared/i18n.js'
import { edit } from './edits.js'
import { run, toast } from '../app/notify.js'
import { invalidateItems } from './results.js'
import { exportFiles as exportTo } from './lifecycle.js'
import { loadDetail } from './selection.js'
import { call, errorText } from '../platform/ipc.js'

const ids = () => [...S.selected]
const details = () => Promise.all(ids().map((id) => call('asset_detail', { id })))

export async function copyTags() {
  const d = await details()
  edit.copyTagNames(d.flatMap((x) => x.tags.map((t) => t.name)))
}
export function pasteTags() {
  edit.pasteTags(ids()).then(loadDetail)
}
export async function customThumb() {
  const p = await call('pick', { kind: 'image' })
  if (p) run('item.setThumbnail', { ids: ids(), path: p[0] }, t('library.thumbnailChanged'))
}
export function clipboardThumb() {
  run('clipboard_thumbnail', { ids: ids() }, t('library.thumbnailChanged'))
}
export function copyFiles() {
  call('copy_files', { ids: ids() }).then((n) => toast(t('library.filesCopied', { count: n }))).catch((e) => toast(errorText(e), 'error'))
}
export function duplicate() {
  toast(t('library.duplicating'))
  run('item.duplicate', { ids: ids() })
}
export function exportFiles() {
  exportTo(ids()).catch(() => {})
}
export function refreshThumbs() {
  invalidateItems(ids())
  call('item.refreshThumbnail', { ids: ids() })
}
export function inNewWindow() {
  call('open_item_window', { ids: ids().slice(0, 200) }).catch((e) => toast(errorText(e), 'error'))
}
