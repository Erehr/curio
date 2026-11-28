// The window's state, `S`, which components read and the modules beside them change, with its defaults.
import { reactive, markRaw } from 'vue'
import { DEFAULT_FOR, defaultGrid } from '../layout/layouts.js'

export const DEFAULT_UI = () => ({
  themeMode: 'dark', darkTheme: 'Dark', lightTheme: 'Light',
  userCss: '', font: '', fontSystem: false, corners: 'normal', spacing: 'normal', customThemes: {}, layouts: {}, layoutFor: { ...DEFAULT_FOR }, grid: defaultGrid(),
  // what the list's font samples and the Fonts viewer write, empty for their own line (view.setSample)
  fontSample: { text: '' },
  // the font viewer's view, text size and background (formats/FontView.vue)
  fontViewer: { view: 'sample', size: 40, look: 'theme' },
  // the tools' settings as last used, and Batch rename's presets (tools/)
  duplicates: null, rename: null, renamePresets: [],
  // Settings › Notifications: OS import notices and notification sounds
  addNotices: 'background', notifySound: true, notifySoundOs: true,
  notifySoundEvents: { import: true, deleteItem: true, deleteFolder: true, error: true, other: false },
  pinnedFilters: ['color', 'tags', 'folder', 'shape', 'rating', 'types', 'ext', 'duration', 'size'],
  spaceAction: 'toggle', rightDragZoom: true,
  // how the mouse and animated images behave (Settings → Controls)
  videoWheel: 'seek', doubleClick: 'here', middleClick: 'window', sideButtons: 'history', gifAlways: false,
  pixelated: false, transparency: false, rememberZoom: false,
  // the text viewer's size, Markdown view (rendered, source or split) and wrapping, the same for every file
  textView: { zoom: 1, mode: 'rendered', wrap: true, lines: false },
  picker: { sidebar: true, starred: true, recent: true },
  recents: {},
  // per library, by folder id: the view settings a folder has of its own, which its subfolders take too
  folderViews: {},
  searchScope: {},
  keys: {},
  viewerWheel: 'navigate',
  // muted until the person turns sound on; the choice is remembered for the whole app
  hoverMuted: true, hoverPlay: true,
  playerVolume: 1, playerMuted: false, playerSpeed: 1, playerEnd: 'repeat',
  viewerStrip: false,
  // bottom, left or right of the open item
  viewerStripSide: 'bottom',
  // thin scrollbars over the content that fade away, rather than the system's own
  overlayScrollbars: true,
  // the Plugins menu: plugins pinned to its top, and the last used
  pluginPins: [],
  pluginRecent: [],
  // Settings: General and Preview; `language` empty to follow the system's
  language: '', zoomLevel: 100, sidebarCounts: true,
  // Settings: General › Window material (Windows 11)
  windowMaterial: 'off', acrylicTransparency: 'medium', windowButtons: true, keepMaterial: false, edgeReveal: false, railOpen: 'hover',
  imageRotation: 'preview', defaultImageSize: 'fit', videoAutoplay: true, videoStretch: false, videoRememberPosition: true,
  hoverAudioAutoplay: true,
  tagView: { layout: 'grid', sort: 'az', groupBy: 'alphabet' },
  // the edit window: tiles or a list
  editView: 'grid',
  // a .css file read after the theme, and read again whenever the window comes back to the front
  themeFile: '',
  // the window shape the main window last had (Settings reads it from its own window)
  aspect: 'horizontal',
  // Settings › Layout: each library keeps its own layouts, view and pinned filters
  layoutPerLibrary: false,
})

// the colour filter's accuracy, 0 to 100, where the query's plain "color ~" sits
export const COLOR_ACCURACY = 50
export function emptyFilters() {
  return {
    kinds: [], kindsNot: [], exts: [], extsNot: [], animated: null, ratings: [], ratingsNot: [], favorite: null, explicit: null, shapes: [], shapesNot: [], ratioW: null, ratioH: null, color: null, colorAccuracy: COLOR_ACCURACY, gray: null, addedWithin: null, modifiedWithin: null, createdWithin: null, addedFrom: null, addedTo: null, modifiedFrom: null, modifiedTo: null, createdFrom: null, createdTo: null,
    sizeMin: null, sizeMax: null, sizeUnit: 'KB', durMin: null, durMax: null, durUnit: 'sec', pagesMin: null, pagesMax: null, preset: null, tags: [], tagsNot: [], tagRule: 'OR', untagged: null, folders: [], foldersNot: [], folderColors: [], folderColorsNot: [], note: '', noteHas: null, url: '', urlHas: null, minW: null, maxW: null, minH: null, maxH: null, fieldConds: [],
    // a field's own filter chip, by key: { values, not, contains, min, max, has }
    fieldPick: {},
  }
}

export const S = reactive({
  ready: false,
  startup: {},
  libOpen: false,
  scope: { kind: 'all', id: 0 },
  back: [],
  fwd: [],
  search: '',
  filters: emptyFilters(),
  // a locked filter stays when moving to another folder
  filterLock: false,
  savedFilters: [],
  // the library's Actions (actions.js)
  actions: [],
  results: markRaw({ ids: new Uint32Array(0), aspects: new Float32Array(0) }),
  resultsVer: 0,
  resultsMs: 0,
  queryError: null,
  searching: false,
  // the search under way asks for something other than what is shown (another folder, query or sort), not for it again after a change
  searchingAnew: false,
  items: markRaw(new Map()),
  itemsVer: 0,
  cells: markRaw(new Map()),
  cellsVer: 0,
  // the Tags screen's selected tags and open group, for its widgets
  tagFocus: null,
  // the sidebar filter's options beside its text: only empty folders, only folders of these colours (null for none)
  sideScope: { empty: null, colors: [] },
  sidebar: { counts: {}, folders: [], tags: [], tagGroups: [], smart: [], roots: [], fields: [], history: [], favorites: [] },
  // locations being scanned, by id: their sidebar rows show it
  scanning: {},
  contrib: { widgets: [], actions: [], filters: [], sorts: [], panels: [], columns: [], tabs: [], viewers: [], viewerTools: [], opensAsFilter: [], formatTypes: [], themeVariables: {} },
  selected: [],
  focus: null,
  anchor: null,
  detail: null,
  view: 'grid',
  // { index, id, file }: `file`, when set, is one of the files grouped into the item that the viewer shows instead of the item's own
  viewer: null,
  // the files the viewer steps through inside the item `item` (loadViewerFiles)
  viewerFiles: { item: null, ids: [] },
  // a viewer page's title and buttons in the item toolbar (viewer.setTools): { frame, title, buttons }
  viewerTools: null,
  // what an action or address opened a plugin's tab for: { key, data, seq }
  tabRun: null,
  status: { busy: null, thumbs: 0 },
  ui: DEFAULT_UI(),
  copiedTags: null,
  // this window kept above the others
  onTop: false,
  dialogs: { prefs: false, prefsTab: 'general', smart: null, libName: null, name: null, icon: null, url: false, confirm: null, choice: null, autoTag: null, action: null, examplePlugin: null },
  // what F2 should start renaming in the sidebar or tag manager
  renameSide: null,
  // bumped when the list of libraries changed outside a library list
  // this computer's libraries, shared by every list and switcher in the window
  libraries: [],
  drag: null,
  toasts: [],
  scrollTo: null,
  // an index the grid scrolls into the middle once its cards are laid out
  revealAt: null,
  systemDark: true,
  recordingKey: false,
  summary: null,
  // popups for adding tags and folders (tags/TagFolderPicker.vue)
  picker: null,
  // what the API asks the person at this window (app/ApiAsk.vue)
  apiAsk: null,
  // sidebar selection: several folders, smart folders or locations
  sideSel: { kind: null, ids: [], anchor: null },
  sideFocus: false,
  sideFilter: '',
  // where a new folder is being named: { parent } or null
  newFolder: null,
  // files being dragged over the window from outside: { folder: name or null }
  fileDrag: null,
  customize: false,
  // the edit mode can show the other window shape's layout, and any screen's
  editAspect: null,
  editScreen: null,
  // the tag manager's search and title, shown by the toolbar's widgets
  tagSearch: '',
  tagTitle: '',
  windowAspect: 'horizontal',
  // this window is Settings
  prefsWindow: false,
  fullscreen: false,
  // a Mac's window buttons are shown (MacWindowButtons; not in full screen)
  lights: false,
  // the window is maximised, on a Mac zoomed
  maximized: false,
  // this window only shows the given items ("Open in New Window")
  itemWindow: false,
  // this window shows a Window plugin's page: { plugin, page }
  pluginWindow: null,
  // this window shows one of Curio's tools (tools/): { tool, run, seq }, `seq` counting the runs it was opened for, with files dropped on it and held over it
  toolWindow: null,
  // counts the themes applied, so plugin pages follow (themeShown)
  themeVer: 0,
})

/** The theme as plugin pages and app.theme get it: every token as its CSS variable takes it. */
export const themeShown = { theme: null, seeThrough: false }

// open/closed state of sidebar folders, kept for the session
export const folderOpen = reactive({})
export const smartOpen = reactive({})
// open locations, and their open folders as "<location id>:<path>"
export const rootOpen = reactive({})
