<script setup>
import { ref } from 'vue'
import { t } from '../shared/i18n.js'
import { S } from '../app/state.js'
import { edit, addToLastFolder, recentOf, readMetadataAgain } from './edits.js'
import { run, toast } from '../app/notify.js'
import { openItem } from './lifecycle.js'
import { listedRoot, rootIcon, showItemIn, startNewFolder } from '../sidebar/sidebar.js'
import { openPicker } from '../tags/tags.js'
import { L } from '../layout/arrangement.js'
import { saveUi } from '../app/preferences.js'
import { call, errorText } from '../platform/ipc.js'
import { actionFits, actionKey, pluginActions, runPluginAction } from '../plugins/pluginActions.js'
import { shortcutLabel } from '../app/shortcuts.js'
import { copyTags, pasteTags, customThumb, clipboardThumb, copyFiles, duplicate, exportFiles, refreshThumbs, inNewWindow } from './itemActions.js'
import AppMenu from '../shared/AppMenu.vue'
import { openTool } from '../tools/tools.js'

// `top`: extra entries for where the menu opens, such as frame commands on a video
const props = defineProps({ top: { type: Function, default: null } })
const cm = ref()
const at = ref(null)
// the item's folders and file, for Show in folder, fetched before the menu opens
let place = null
let hasTags = true
defineExpose({
  show: async (e) => {
    const ev = { type: e.type, clientX: e.clientX, clientY: e.clientY, currentTarget: e.currentTarget, preventDefault() {}, stopPropagation() {} }
    at.value = { clientX: e.clientX, clientY: e.clientY }
    const id = S.selected.length === 1 ? first() : null
    const d = id == null ? null : S.detail?.asset?.id === id ? S.detail : await call('asset_detail', { id }).catch(() => null)
    place = d ? { folders: d.folders || [], asset: d.asset } : null
    if (id != null) hasTags = (d?.tags?.length ?? 0) > 0
    else {
      const s = S.summary?.count === S.selected.length ? S.summary : await call('selection_summary', { ids: S.selected.slice(0, 200000) }).catch(() => null)
      hasTags = s ? !!s.anyTag : true
    }
    cm.value.show(ev)
  },
})

const ids = () => [...S.selected]
const first = () => S.selected[S.selected.length - 1]
const sc = (id) => shortcutLabel(S.ui.keys, id)
const item0 = () => S.items.get(first())
const allReadOnly = () => S.selected.every((id) => S.items.get(id)?.read_only)
const sel = () => S.selected.map((id) => S.items.get(id)).filter(Boolean)
const someLinked = () => sel().some((it) => !it.managed)
const someMovable = () => sel().some((it) => it.managed || it.organize)
const someOrganized = () => sel().some((it) => !it.managed && it.organize)
const isLink = () => ['url', 'webloc'].includes(item0()?.ext)

async function copyText(fn, msg) {
  await navigator.clipboard.writeText(await fn())
  toast(msg)
}
const paths = () => call('paths_of', { ids: ids() })
const details = () => Promise.all(ids().map((id) => call('asset_detail', { id })))

async function replaceWithFile() {
  const id = first()
  const p = await call('pick', { kind: 'file' })
  if (!p) return
  S.dialogs.confirm = {
    title: t('library.menu.replace.title'),
    body: t('library.menu.replace.body', { name: p[0].split(/[\\/]/).pop() }),
    okLabel: t('library.menu.replace.ok'),
    ok: () => run('item.replaceFile', { id, path: p[0] }, t('library.menu.replace.done')),
  }
}

// the item right-clicked takes the others as its files
function groupSelected() {
  const item = first()
  run('item.group', { id: item, members: ids().filter((x) => x !== item) }, t('library.menu.grouped'))
}

// the item in Curio: its folders and, indexed in place, its folder in the location, with All when there are several; where the view already is can't be chosen
function locationEntry(many) {
  const id = first()
  const entry = { label: t('library.menu.items.showInFolder'), icon: 'file-search', visible: !many && S.scope.kind !== 'trash' && !S.itemWindow }
  const places = (place?.folders || []).map((f) => ({ label: f.name, icon: f.icon || 'folder', color: f.color, at: { kind: 'folder', id: f.id } }))
  const a = place?.asset
  const root = a && S.sidebar.roots.find((r) => r.id === a.root_id && listedRoot(r))
  if (root) {
    const dir = a.rel_path.includes('/') ? a.rel_path.slice(0, a.rel_path.lastIndexOf('/')) : ''
    places.push(dir ? { label: dir.split('/').pop(), icon: 'folder', at: { kind: 'rootdir', id: root.id, path: dir } } : { label: root.label, icon: rootIcon(root), color: root.color, at: { kind: 'root', id: root.id } })
  }
  const all = { label: t('library.views.all'), icon: 'inbox', at: { kind: 'all' } }
  const here = (at) => S.scope.kind === at.kind && (at.kind === 'all' || (S.scope.id === at.id && (at.kind !== 'rootdir' || S.scope.path === at.path)))
  const row = (p) => ({ label: p.label, icon: p.icon, color: p.color, disabled: here(p.at), command: () => showItemIn(id, p.at) })
  if (places.length > 1) entry.items = [all, ...places].map(row)
  else Object.assign(entry, { disabled: here((places[0] || all).at), command: row(places[0] || all).command })
  return entry
}

function model() {
  const many = S.selected.length > 1
  const it = item0()
  const list = props.top ? [...props.top()] : []
  list.push({ label: S.selected.length > 1 ? t('library.menu.openThese') : t('library.menu.items.openWindow'), icon: 'app-window', shortcut: sc('item.newWindow'), command: inNewWindow })
  list.push({ label: t('library.menu.items.openExternal'), icon: 'external-link', shortcut: sc('item.openExternal'), command: () => ids().slice(0, 10).forEach((id) => openItem(id, 'open')) })
  list.push({ label: t('library.menu.items.openIn'), icon: 'apps', visible: !many, command: () => call('open_with', { id: first() }).catch((e) => toast(errorText(e), 'error')) })
  list.push({ label: t('library.menu.items.reveal'), icon: 'folder-open', shortcut: sc('item.reveal'), visible: !many, command: () => openItem(first(), 'reveal') })
  list.push(locationEntry(many))
  if (!many && isLink()) {
    list.push({ label: t('library.menu.items.openHere'), icon: 'world', command: async () => { const d = await call('asset_detail', { id: first() }); call('open_web', { url: d.asset.source_url, title: it?.title }) } })
    list.push({ label: t('library.menu.items.openBrowser'), icon: 'world-share', command: async () => { const d = await call('asset_detail', { id: first() }); call('open_url', { url: d.asset.source_url }) } })
  }
  list.push({ separator: true })
  // every plugin action for the menu, so the submenu is always where it is; those not for these items (kinds, how many) are greyed
  const plugins = pluginActions('item-menu')
  if (plugins.length) {
    list.push({ label: t('library.menu.items.plugins'), icon: 'puzzle', items: plugins.map((a) => ({ label: a.title, icon: a.icon || 'puzzle', shortcut: actionKey(a).split('+').join(' + '), disabled: !!a.disabled || !actionFits(a, ids()), command: () => runPluginAction(a, ids()).catch(() => {}) })) })
    list.push({ separator: true })
  }
  list.push({ label: t('library.menu.items.toLastFolder'), icon: 'folder-up', shortcut: sc('item.toLastFolder'), disabled: !recentOf('folders').length, command: () => addToLastFolder(ids()) })
  list.push({ label: t('library.menu.items.toFolders'), icon: 'folder-plus', shortcut: sc('item.toFolders'), command: () => openPicker('folders', at.value) })
  list.push({ label: t('library.menu.items.newFolder'), icon: 'folder-star', command: () => edit.folder('create', { name: t('library.newFolderName'), ids: ids() }) })
  list.push({ label: t('library.menu.items.addTags'), icon: 'tag', shortcut: sc('tags.add'), command: () => openPicker('tags', at.value) })
  const others = S.libraries.filter((x) => !x.current && x.exists)
  list.push({ label: t('library.menu.items.copyTo'), icon: 'copy', visible: others.length > 0, items: others.map((l) => ({ label: l.name, icon: 'books', command: () => edit.sendTo(l, ids(), false) })) })
  list.push({ label: t('library.menu.items.moveTo'), icon: 'corner-up-right', visible: others.length > 0, items: others.map((l) => ({ label: l.name, icon: 'books', command: () => edit.sendTo(l, ids(), true) })) })
  const allFav = sel().length > 0 && sel().every((x) => x.favorite)
  const allExp = sel().length > 0 && sel().every((x) => x.explicit)
  list.push({
    label: t('library.menu.items.rating'), icon: 'star',
    items: [
      ...[5, 4, 3, 2, 1, 0].map((r) => ({ label: r ? '★'.repeat(r) : t('library.menu.ratingNone'), current: sel().length > 0 && sel().every((x) => (x.rating || 0) === r), shortcut: String(r), command: () => edit.rating(ids(), r) })),
      { separator: true },
      { label: t('inspector.favorite'), icon: allFav ? 'heart-filled' : 'heart', checked: allFav, shortcut: sc('rating.favorite'), command: () => edit.favorite(ids(), !allFav) },
      { label: t('inspector.explicit'), icon: 'shield-exclamation', checked: allExp, shortcut: sc('rating.explicit'), command: () => edit.explicit(ids(), !allExp) },
    ],
  })
  list.push({
    label: t('library.menu.items.export'), icon: 'upload',
    items: [
      { label: t('library.menu.items.exportFiles'), icon: 'file-export', shortcut: sc('item.export'), command: exportFiles },
      { separator: true },
      { label: t('library.menu.items.json'), icon: 'braces', command: () => run('export', { kind: 'json', ids: ids() }) },
      { label: t('library.menu.items.csv'), icon: 'table-export', command: () => run('export', { kind: 'csv', ids: ids() }) },
      { label: t('library.menu.items.sidecarFolder'), icon: 'files', command: () => run('export', { kind: 'sidecar-folder', ids: ids() }) },
      { label: t('library.menu.items.sidecarBeside'), icon: 'file-plus', visible: !allReadOnly(), command: () => (S.dialogs.confirm = { title: t('library.menu.sidecars.title'), body: t('library.menu.sidecars.body'), okLabel: t('library.menu.sidecars.ok'), okSeverity: 'primary', ok: () => run('export', { kind: 'sidecar-beside', ids: ids() }) }) },
    ],
  })
  list.push({
    label: t('library.menu.items.storage'), icon: 'database',
    visible: someLinked() || someMovable(),
    items: [
      { label: t('library.menu.items.copyIn'), icon: 'copy-plus', visible: someLinked(), command: () => run('transfer', { action: 'copy_in', ids: ids() }) },
      {
        label: t('library.menu.items.moveIn'), icon: 'arrow-bar-to-down', visible: someOrganized(),
        command: () => edit.moveIn(ids()),
      },
      { label: t('library.menu.items.toDiskFolder'), icon: 'folder-share', visible: someMovable(), command: () => run('transfer', { action: 'to_folder', ids: ids() }) },
    ],
  })
  list.push({ separator: true })
  list.push({ label: t('library.menu.items.rename'), icon: 'cursor-text', shortcut: sc('item.rename'), visible: !many, command: () => document.getElementById('item-name')?.focus() })
  list.push({ label: t('library.menu.items.renameAll'), icon: 'cursor-text', shortcut: sc('item.rename'), visible: many, command: () => openTool('rename', ids()) })
  list.push({ label: t('library.menu.items.findDuplicates'), icon: 'copy-check', command: () => openTool('duplicates', ids(), { action: 'find' }) })
  list.push({ label: t('library.menu.items.compare'), icon: 'layout-columns', visible: many && sel().length === S.selected.length && sel().every((x) => x.kind === 'image'), command: () => openTool('compare', ids().slice(0, 2)) })
  list.push({
    label: t('library.menu.items.copy'), icon: 'copy',
    items: [
      { label: t('library.menu.items.copyFiles'), icon: 'files', shortcut: sc('item.copy'), command: copyFiles },
      { label: t('library.menu.items.copyPath'), icon: 'terminal-2', shortcut: sc('item.copyPath'), command: () => copyText(async () => (await paths()).join('\n'), t('library.menu.copied.path')) },
      { label: t('library.menu.items.copyFolder'), icon: 'folder', command: () => copyText(async () => [...new Set((await paths()).map((p) => p.replace(/[\\/][^\\/]*$/, '')))].join('\n'), t('library.menu.copied.folder')) },
      { separator: true },
      { label: t('library.menu.items.copyName'), icon: 'letter-case', command: () => copyText(async () => (await details()).map((d) => d.asset.display_name || d.asset.name).join('\n'), t('library.menu.copied.name')) },
      { label: t('library.menu.items.copyLink'), icon: 'link', command: () => copyText(async () => (await details()).map((d) => d.asset.source_url).filter(Boolean).join('\n'), t('library.menu.copied.link')) },
      { label: t('library.menu.items.copyNotes'), icon: 'notes', command: () => copyText(async () => (await details()).map((d) => d.asset.annotation).filter(Boolean).join('\n'), t('library.menu.copied.notes')) },
      { separator: true },
      { label: t('library.menu.items.copyTags'), icon: 'bookmarks', shortcut: sc('tags.copy'), command: copyTags },
    ],
  })
  list.push({ label: t('library.menu.items.pasteTags'), icon: 'clipboard-plus', shortcut: sc('tags.paste'), disabled: !S.copiedTags?.length, command: pasteTags })
  list.push({ label: t('library.menu.items.removeTags'), icon: 'bookmark-off', disabled: !hasTags, command: () => edit.clearTags(ids()) })
  list.push({ separator: true })
  list.push({ label: t('library.menu.items.duplicate'), icon: 'copy-plus', shortcut: sc('item.duplicate'), command: duplicate })
  list.push({ label: t('library.menu.items.group'), icon: 'stack-push', visible: many, command: groupSelected })
  list.push({ label: t('library.menu.items.split'), icon: 'arrows-split-2', visible: sel().some((x) => x.parts > 0), command: () => run('item.split', { ids: sel().filter((x) => x.parts > 0).map((x) => x.id) }, t('library.menu.splitDone')) })
  list.push({ label: t('library.menu.items.allowGrouping'), icon: 'stack-2', visible: sel().some((x) => x.solo), command: () => run('item.allowGrouping', { ids: sel().filter((x) => x.solo).map((x) => x.id) }) })
  list.push({ separator: true })
  const lay = L()
  list.push({
    label: t('library.menu.items.backdrop'), icon: 'background',
    items: ['auto', 'dark', 'light'].map((v) => ({ label: t(`library.menu.backdrops.${v}`), current: lay.thumbBg === v, command: () => { lay.thumbBg = v; saveUi() } })),
  })
  list.push({ label: t('library.menu.items.gray'), icon: 'contrast', shortcut: sc('look.gray'), checked: !!lay.grayscale, command: () => { lay.grayscale = !lay.grayscale; saveUi() } })
  list.push({
    label: t('library.menu.items.more'), icon: 'dots',
    items: [
      { label: t('library.menu.items.thumbFile'), icon: 'photo-edit', shortcut: sc('thumb.fromFile'), command: customThumb },
      { label: t('library.menu.items.thumbClipboard'), icon: 'clipboard', shortcut: sc('thumb.fromClipboard'), command: clipboardThumb },
      { separator: true },
      { label: t('library.menu.items.thumbRefresh'), icon: 'refresh', shortcut: sc('thumb.refresh'), command: refreshThumbs },
      { label: t('library.menu.items.refreshColors'), icon: 'palette', command: () => run('item.refreshPalette', { ids: ids() }, t('library.menu.colorsUpdated')) },
      { label: t('library.menu.items.rescan'), icon: 'file-search', command: () => readMetadataAgain(ids()) },
      { label: t('library.menu.items.replaceFile'), icon: 'file-import', visible: !many && !it?.read_only, command: replaceWithFile },
      { label: t('library.menu.items.locate'), icon: 'link', visible: !many && it && it.status !== 'Ok', command: () => run('relink', { id: first() }) },
      { label: t('library.menu.items.newSubfolder'), icon: 'folder-plus', visible: S.scope.kind === 'folder', command: () => startNewFolder(S.scope.id) },
    ],
  })
  list.push({ separator: true })
  if (S.scope.kind === 'trash') {
    list.push({ label: t('library.menu.items.restore'), icon: 'restore', command: () => edit.trash(ids(), false) })
    list.push({ label: t('library.menu.items.purge'), icon: 'trash-x', danger: true, shortcut: sc('item.trash'), command: () => edit.purge(ids()) })
  }
  else list.push({ label: t('library.menu.items.trash'), icon: 'trash', shortcut: sc('item.trash'), command: () => edit.trash(ids(), true) })
  return list
}
</script>

<template>
  <AppMenu ref="cm" :model="model" searchable />
</template>
