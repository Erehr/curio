// What a key does in the window; which key runs which action, and the defaults, are in shortcuts.js.
import { getCurrentWindow } from '../platform/windowFrame.js'
import { t } from '../shared/i18n.js'
import { S } from './state.js'
import { L, cycleLibrary, cycleInspector } from '../layout/arrangement.js'
import { saveUi, openPrefs } from './preferences.js'
import { selectAll, clearSelection, selectOnly, selectRange } from '../library/selection.js'
import { setScope, goBack, goForward, resetFilters } from '../library/navigation.js'
import { edit, addToLastFolder } from '../library/edits.js'
import { toast } from './notify.js'
import { openViewer, stepViewer, stepFile } from '../viewer/session.js'
import { openItem, pasteClipboard } from '../library/lifecycle.js'
import { openPicker } from '../tags/tags.js'
import { startNewFolder, deleteSide, setAllFoldersOpen } from '../sidebar/sidebar.js'
import { actionFor, typingAllowed, comboOf, normalise, FROM_PAGE } from './shortcuts.js'
import { runUserAction } from '../actions/actions.js'
import { pluginActionFor, runPluginAction, viewerKeyFor } from '../plugins/pluginActions.js'
import { AUDIO_SPEEDS, VIDEO_SPEEDS, copyFrame, saveFrame } from '../formats/playerState.js'
import { fit, flip, nextTextMode, rotate, scrollText, toggleText, viewerMounted, zoomStep } from '../viewer/viewerControls.js'
import { neighbor } from '../shared/gridLayout.js'
import { call, errorText } from '../platform/ipc.js'
import { viewerKeyPressed, viewerPlugin } from '../plugins/bridge.js'
import { openTool } from '../tools/tools.js'
import { copyTags, pasteTags, customThumb, clipboardThumb, copyFiles, duplicate, exportFiles, refreshThumbs, inNewWindow } from '../library/itemActions.js'

const RATED_NOTICE_MS = 1000
// the item grid's component, null while another view shows
let grid = () => null

// What the player's shortcuts do.
const VOLUME_STEP = 0.1
// a skip is a tenth of a short clip, else a few seconds
const STEP_SHARE = 0.1
const STEP_MAX_SECONDS = 5
const openMedia = () => document.querySelector('.viewer video, .viewer audio')
function runPlayerAction(id, m) {
  const speeds = m.tagName === 'AUDIO' ? AUDIO_SPEEDS : VIDEO_SPEEDS
  const step = Math.min((m.duration || 0) * STEP_SHARE, STEP_MAX_SECONDS)
  const volume = (d) => {
    S.ui.playerVolume = Math.max(0, Math.min(1, (S.ui.playerVolume ?? 1) + d))
    saveUi()
  }
  const speed = (d) => {
    const next = speeds[speeds.indexOf(S.ui.playerSpeed || 1) + d]
    if (next) {
      S.ui.playerSpeed = next
      saveUi()
    }
  }
  const frames = (n) => {
    m.pause()
    m.currentTime += n * S.startup.frameSeconds
  }
  switch (id) {
    case 'player.copyFrame': return m.tagName === 'VIDEO' ? copyFrame() : undefined
    case 'player.saveFrame': return m.tagName === 'VIDEO' ? saveFrame() : undefined
    case 'player.louder': return volume(VOLUME_STEP)
    case 'player.quieter': return volume(-VOLUME_STEP)
    case 'player.seekAhead': m.currentTime += step; return
    case 'player.seekBack': m.currentTime -= step; return
    case 'player.faster': return speed(1)
    case 'player.slower': return speed(-1)
    case 'player.frameBack': return frames(-1)
    case 'player.frameAhead': return frames(1)
    case 'player.tenBack': return frames(-10)
    case 'player.tenAhead': return frames(10)
  }
}

function runAction(id) {
  const l = L()
  const sel = () => [...S.selected]
  const last = () => S.selected[S.selected.length - 1]
  const layoutOf = { 'layout.masonry': 'masonry', 'layout.justified': 'justified', 'layout.grid': 'grid', 'layout.list': 'list', 'layout.spectrum': 'spectrum' }
  const scopes = { 'go.all': 'all', 'go.uncategorized': 'uncategorized', 'go.untagged': 'untagged', 'go.recent': 'recent', 'go.random': 'random', 'go.trash': 'trash' }
  if (layoutOf[id]) {
    l.view = layoutOf[id]
    return saveUi()
  }
  if (scopes[id]) return setScope(scopes[id])
  if (id === 'rating.favorite' || id === 'rating.explicit') {
    if (S.selected.length) {
      const ids = sel()
      const key = id === 'rating.favorite' ? 'favorite' : 'explicit'
      const allOn = ids.every((x) => (S.items.get(x) || (S.detail?.asset?.id === x ? S.detail.asset : null))?.[key])
      edit[key](ids, !allOn)
    }
    return
  }
  if (id.startsWith('rating.')) {
    if (S.selected.length) {
      const stars = Number(id.slice(-1))
      edit.rating(sel(), stars)
      toast(t('app.rated', { count: stars }), 'info', RATED_NOTICE_MS)
    }
    return
  }
  switch (id) {
    case 'item.actions': return document.querySelector('[data-action-btn]')?.click()
    case 'thumb.fromFile': if (S.selected.length) customThumb(); return
    case 'thumb.fromClipboard': if (S.selected.length) clipboardThumb(); return
    case 'folder.defaultTags': {
      const f = S.scope.kind === 'folder' && S.sidebar.folders.find((x) => x.id === S.scope.id)
      if (f) S.dialogs.autoTag = { id: f.id, name: f.name, tags: [...(f.auto_tags || [])] }
      return
    }
    case 'viewer.rotate': return viewerMounted() ? rotate() : undefined
    case 'viewer.flip': return viewerMounted() ? flip() : undefined
    case 'viewer.fileBack': return stepFile(-1)
    case 'viewer.fileAhead': return stepFile(1)
    case 'panel.left': return cycleLibrary()
    case 'panel.right': return cycleInspector()
    case 'panel.both': { const both = l.library || l.inspector; l.library = !both; l.inspector = !both; return saveUi() }
    case 'go.tags': S.view = 'tags'; S.viewer = null; return
    case 'cards.larger': if (S.viewer) return zoomStep(1); l.size = Math.min(640, Math.round(l.size * 1.15)); return saveUi()
    case 'cards.smaller': if (S.viewer) return zoomStep(-1); l.size = Math.max(60, Math.round(l.size / 1.15)); return saveUi()
    case 'viewer.zoomReset': return S.viewer ? fit() : undefined
    case 'text.mode': return nextTextMode()
    case 'text.wrap': return toggleText('wrap')
    case 'text.lines': return toggleText('lines')
    case 'scroll.top': if (S.viewer) return void scrollText('top'); S.scrollTo = 0; return
    case 'scroll.bottom': if (S.viewer) return void scrollText('bottom'); S.scrollTo = 1e12; return
    case 'look.gray': l.grayscale = !l.grayscale; return saveUi()
    case 'nav.back': return goBack()
    case 'nav.forward': return goForward()
    case 'search.focus': return document.getElementById('search-box')?.focus()
    case 'filters.toggle': l.filterbar = !l.filterbar; return saveUi()
    case 'filters.clear': resetFilters(); return
    case 'sidebar.filter': {
      const box = document.getElementById('side-filter')
      if (box) box.focus()
      else document.getElementById('side-filter-btn')?.click()
      return
    }
    case 'item.toFolders': return S.selected.length ? openPicker('folders', null) : undefined
    case 'item.open':
      if (S.ui.spaceAction === 'scroll' && !S.viewer) return grid()?.pageDown?.(1)
      // in a player Space plays and pauses
      if (S.viewer && S.ui.spaceAction !== 'hold') {
        const video = document.querySelector('.viewer video, .viewer audio')
        if (video) {
          if (video.paused) video.play()
          else video.pause()
          return
        }
      }
      if (S.viewer) S.viewer = null
      else if (S.selected.length && S.focus != null) openViewer(S.focus, last())
      return
    case 'item.openExternal': if (last() != null) openItem(last(), 'open'); return
    case 'item.reveal': if (last() != null) openItem(last(), 'reveal'); return
    case 'item.newWindow': if (last() != null) inNewWindow(); return
    case 'item.rename': {
      const s = S.sideSel
      if (S.view === 'tags') S.renameSide = { kind: 'tagGroup' }
      else if (S.sideFocus && s.kind && s.ids.length === 1 && s.kind !== 'root') S.renameSide = { kind: s.kind, id: s.ids[0] }
      else if (S.selected.length === 1) document.getElementById('item-name')?.focus()
      else if (S.selected.length > 1) openTool('rename', [...S.selected])
      return
    }
    case 'tags.pick': return S.selected.length ? openPicker('tags', null) : undefined
    case 'folders.pick': return S.selected.length ? openPicker('folders', null) : undefined
    case 'folders.openAll': return setAllFoldersOpen()
    case 'item.copyPath': call('paths_of', { ids: sel() }).then((p) => navigator.clipboard.writeText(p.join('\n')).then(() => toast(t('app.copiedPaths', { count: p.length })))); return
    case 'item.copy': if (S.selected.length) copyFiles(); return
    case 'item.duplicate': if (S.selected.length) duplicate(); return
    case 'item.selectAll': return selectAll()
    case 'item.paste': return pasteClipboard()
    case 'item.trash': {
      // Delete applies to whichever side was used last
      const s = S.sideSel
      if (S.sideFocus && s.kind && s.ids.length) return deleteSide(s.kind, [...s.ids])
      return S.scope.kind === 'trash' ? edit.purge(sel()) : edit.trash(sel(), true)
    }
    case 'thumb.refresh': if (S.selected.length) refreshThumbs(); return
    case 'tags.add': return S.selected.length ? openPicker('tags', null) : undefined
    case 'tags.copy': if (S.selected.length) copyTags(); return
    case 'tags.paste': if (S.selected.length) pasteTags(); return
    case 'item.toLastFolder': if (S.selected.length) addToLastFolder(sel()); return
    case 'folder.new': return startNewFolder()
    case 'folder.newInside': return startNewFolder(S.scope.kind === 'folder' ? S.scope.id : null)
    case 'smart.new': S.dialogs.smart = { id: null, name: t('filters.smart.newName'), query: '' }; return
    case 'links.add': S.dialogs.url = true; return
    case 'item.export': if (S.selected.length) exportFiles(); return
    case 'library.choose': return document.getElementById('lib-switch')?.click()
    case 'app.settings': return openPrefs()
    case 'app.quit': call('quit_app').catch((e) => toast(errorText(e), 'error')); return
  }
}

// Settings → Controls: "open only while Space is held"
function onKeyUp(e) {
  if (e.key === ' ' && S.ui.spaceAction === 'hold' && S.viewer) S.viewer = null
}

const FILE_STEPS = { 'viewer.fileBack': -1, 'viewer.fileAhead': 1 }
const textSelected = () => {
  const sel = window.getSelection()
  return !!sel && !sel.isCollapsed && !!sel.anchorNode?.parentElement?.closest('.text-view')
}
// A modal has the keys while it is open, the Escape that closes it too.
// OpenVue's Dialog closes on Escape from a document listener, before onKey hears the key, so the modal is looked for as the key goes down.
// A closing one has let the keys go: its mask stays until its leave transition ends, which waits for an animation frame a window behind others never gets.
let modalKey = null
const noteModal = (e) => (modalKey = document.querySelector('.p-overlay-mask:not(.p-overlay-mask-leave-active)') ? e : null)

function onKey(e) {
  if (e === modalKey || S.recordingKey || S.picker || S.dialogs.action || S.pluginWindow || S.toolWindow) return
  const t = e.target
  // a focused select or list takes letters and digits as type-ahead
  const typing = t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable || !!t.closest?.('[role=combobox], [role=listbox], .p-select, .p-listbox'))
  const fromPage = e.fromPage === true
  // a viewer page's own keys, over every other while it shows
  if (!typing && S.viewer) {
    const k = viewerKeyFor(e, viewerPlugin())
    if (k) {
      e.preventDefault()
      return viewerKeyPressed(k.id)
    }
  }
  // an Action's own shortcut
  if (!typing && !fromPage && S.actions.length) {
    const c = comboOf(e)
    const a = c && S.actions.find((x) => x.shortcut && normalise(x.shortcut) === c)
    if (a) {
      e.preventDefault()
      return runUserAction(a)
    }
  }
  // a plugin action's key, while the selected items have the focus, over Curio's own
  if (!typing && !fromPage && !S.sideFocus && S.view === 'grid' && S.contrib.actions?.length) {
    const a = pluginActionFor(e, S.selected)
    if (a) {
      e.preventDefault()
      runPluginAction(a, [...S.selected]).catch(() => {})
      return
    }
  }
  const id = actionFor(e, S.ui.keys || {})
  if (fromPage && id && !FROM_PAGE.has(id)) return
  // text selected in the viewer copies as text, not as files
  if (id === 'item.copy' && S.viewer && !typing && textSelected()) return
  if (FILE_STEPS[id]) {
    // these keys page the grid unless the viewer shows an item holding several files
    if (!typing && S.viewer && stepFile(FILE_STEPS[id])) {
      e.preventDefault()
      return
    }
  } else if (id?.startsWith('player.')) {
    // these keys mean something else unless a video or song is open
    const m = openMedia()
    if (m && !typing) {
      e.preventDefault()
      runPlayerAction(id, m)
      return
    }
  } else if (id && (!typing || typingAllowed(id))) {
    e.preventDefault()
    if (e.repeat && id === 'item.open') return
    runAction(id)
    return
  }
  if (typing) return
  const k = e.key
  if (k === 'Escape') {
    if (S.customize) {
      S.customize = false
      S.editAspect = null
    }
    else if (S.fullscreen) S.fullscreen = false
    else if (S.itemWindow) getCurrentWindow().close().catch(() => {})
    else if (S.viewer) S.viewer = null
    else if (S.sideSel.ids.length > 1) {
      S.sideSel = { kind: null, ids: [], anchor: null }
      setScope('all')
    } else clearSelection()
  } else if (k === 'Enter' && !e.shiftKey && !e.ctrlKey) {
    runAction('item.open')
    e.preventDefault()
  } else if (k === 'PageDown' || k === 'PageUp') {
    if (S.viewer) scrollText(k === 'PageDown' ? 1 : -1)
    else grid()?.pageDown?.(k === 'PageDown' ? 1 : -1)
    e.preventDefault()
  } else if (k.startsWith('Arrow') && !e.altKey) {
    const dx = k === 'ArrowRight' ? 1 : k === 'ArrowLeft' ? -1 : 0
    const dy = k === 'ArrowDown' ? 1 : k === 'ArrowUp' ? -1 : 0
    if (S.viewer) {
      stepViewer(dx || dy)
      grid()?.reveal?.(S.viewer?.index)
    } else {
      const next = neighbor(grid()?.layout?.(), S.focus, dx, dy)
      if (next != null) {
        const nid = S.results.ids[next]
        if (e.shiftKey) selectRange(next, grid()?.layout?.())
        else selectOnly(nid, next)
        grid()?.reveal?.(next)
      }
    }
    e.preventDefault()
  }
}

// a mouse's side buttons go back and forward or step through items, as Settings › Controls says, rather than moving the web view through its own history
function onSideButton(e) {
  if (e.button !== 3 && e.button !== 4) return
  e.preventDefault()
  if (e.type !== 'mouseup' || S.recordingKey || S.picker || S.pluginWindow || S.toolWindow || S.itemWindow || S.prefsWindow || document.querySelector('.p-overlay-mask')) return
  const mode = S.ui.sideButtons || 'history'
  if (mode === 'none') return
  if (mode === 'navigate') {
    const d = e.button === 3 ? -1 : 1
    if (S.viewer) {
      stepViewer(d)
      grid()?.reveal?.(S.viewer?.index)
    } else if (S.results.ids.length) {
      const cur = S.focus ?? (d > 0 ? -1 : S.results.ids.length)
      const next = Math.max(0, Math.min(S.results.ids.length - 1, cur + d))
      if (next !== cur) {
        selectOnly(S.results.ids[next], next)
        grid()?.reveal?.(next)
      }
    }
    return
  }
  if (e.button === 3) goBack()
  else goForward()
}

export function installKeyboard(inputs) {
  grid = inputs.grid
  window.addEventListener('keydown', noteModal, true)
  window.addEventListener('keydown', onKey)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('mousedown', onSideButton)
  window.addEventListener('mouseup', onSideButton)
  return () => {
    window.removeEventListener('keydown', noteModal, true)
    window.removeEventListener('keydown', onKey)
    window.removeEventListener('keyup', onKeyUp)
    window.removeEventListener('mousedown', onSideButton)
    window.removeEventListener('mouseup', onSideButton)
  }
}
