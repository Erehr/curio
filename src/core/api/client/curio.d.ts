// Curio's web API for TypeScript and editors: the client in curio.js with every operation, its parameters and its answer.
// Generated from Curio's operation registry (src/core/api/reference.rs); served at http://127.0.0.1:41733/api/v1/curio.d.ts. Don't edit it by hand.
// In a script: /// <reference path="curio.d.ts" />

declare namespace Curio {
  interface Options {
    /** A token from Settings › API. */
    token: string;
    /** 41733 unless changed in Settings › API. */
    port?: number;
    host?: string;
    /** The library calls go to when they name none: an id from library.list, a folder or a name. */
    library?: string;
    /** fetch, or gm for GM_xmlhttpRequest in a userscript; found by itself when left out. Events use GM_xmlhttpRequest only in Tampermonkey, or with gm. */
    transport?: 'fetch' | 'gm';
  }

  interface CurioError extends Error {
    /** bad_request, unauthorized, not_allowed, not_found, unknown_operation, unknown_library, library_closed, library_offline, no_window, busy, failed, or network when Curio couldn't be reached. */
    code: string;
    /** The HTTP status; 0 when Curio couldn't be reached. */
    status: number;
  }

  interface EventFilter {
    /** Names, or a namespace as items.*; every event the token may receive when left out. */
    events?: (keyof EventMap | `${string}.*`)[];
    /** One library's events; the library list's still come. */
    library?: string;
    /** These items' events; needs library. */
    ids?: string[];
  }

  interface Event<N extends keyof EventMap = keyof EventMap> {
    id: string;
    name: N;
    /** The library's id; null for the library list. */
    library: string | null;
    /** Milliseconds since 1970. */
    at: number;
    data: EventMap[N];
  }

  interface Subscription {
    /** Ends the stream; it isn't opened again. */
    close(): void;
  }

  interface Client extends Api {
    /** Any operation by its name, for names this file doesn't know yet. */
    call(op: string, params?: Record<string, unknown>): Promise<any>;
    /** Follows events; the stream reconnects by itself, resuming after the last event, until closed. */
    subscribe(filter: EventFilter, onEvent: (e: Event) => void, onState?: (state: 'open' | 'retrying' | 'closed', error?: CurioError) => void): Subscription;
    on<N extends keyof EventMap>(name: N, onEvent: (e: Event<N>) => void, filter?: Omit<EventFilter, 'events'>): Subscription;
    /** The same client with calls going to another library when they name none. */
    forLibrary(library: string): Client;
  }

  /** The call was done; nothing more to tell. */
  type Done = { success: true };

  /** An entry of this computer's library list. */
  interface Library {
    /** Stable across renames and moves; null until the library was first open in Curio 0.7 or later. */
    id: string | null;
    name: string;
    icon: string | null;
    color: string | null;
    /** window: a window shows it; background: a process without a window holds it; closed: a call opens it in the background; offline: its folder can't be reached. */
    state: 'window' | 'background' | 'closed' | 'offline';
    /** Where calls that name no library go. */
    default: boolean;
    /** The id of the window showing it, for window operations. */
    window?: string;
    /** With files.read. */
    path?: string;
  }

  /** A window's theme. `tokens` maps each CSS variable's name, without the leading dashes, to its value: colours such as `bg`, `panel`, `surface`, `text` and `accent`, the scales `r` (corners), `round` (the corners of what is otherwise fully round), `g` (spacing) and `row`, and the fonts `ui-font` and `mono-font`. */
  interface Theme {
    name: string;
    dark: boolean;
    tokens: Record<string, string>;
  }

  interface LibraryInfo {
    id: string | null;
    name: string | null;
    schema: number;
    floor: number;
    counts: { all: number; favorites: number; uncategorized: number; untagged: number; trash: number; missing: number; recent: number; tags: number; size: number };
    path?: string;
  }

  type Kind = 'image' | 'video' | 'audio' | 'document' | 'code' | 'link' | 'archive' | 'font' | 'model' | 'other';

  interface ItemMember {
    id: string;
    role: string | null;
    name: string;
    ext: string;
    size: number;
    status: 'Available' | 'Storage offline' | 'Missing';
  }

  interface Item {
    /** A UUID, stable across moves, renames and relinks. */
    id: string;
    name: string;
    fileName: string;
    ext: string;
    kind: Kind;
    size: number;
    width: number | null;
    height: number | null;
    /** Seconds. */
    duration: number | null;
    /** 0 to 5. */
    rating: number;
    favorite: boolean;
    explicit: boolean;
    annotation: string;
    /** Where it came from; for a link, the link. */
    url: string;
    tags: string[];
    folders: { id: number; name: string }[];
    status: 'Available' | 'Storage offline' | 'Missing';
    /** Milliseconds since 1970. */
    imported: number;
    modified: number | null;
    /** Number of grouped member files. */
    parts: number;
    /** Parent item UUID when this asset is a grouped member. */
    partOf?: string;
    partRole?: string | null;
    /** With files.read. */
    path?: string;
  }

  interface ItemWithFields extends Omit<Item, 'parts'> {
    /** Custom field values by key. */
    fields: Record<string, unknown>;
    /** 0 when ungrouped, or member summaries when grouped. */
    parts: number | ItemMember[];
  }

  interface Page<T> {
    total: number;
    offset: number;
    items: T[];
  }

  /** New items, or `queued` when adding them took longer than the call waits. */
  type Added = { success: true; ids: string[]; errors: string[]; handed?: { url: string; plugin: string; handler: string }[] } | { success: true; queued: true };

  interface Folder {
    id: number;
    parentId: number | null;
    name: string;
    color: string | null;
    sortOrder: number;
    /** Items in it, not counting subfolders. */
    count: number;
    icon: string | null;
    /** Tags its new items get. */
    autoTags: string[];
  }

  interface SmartFolder {
    id: number;
    name: string;
    query: string;
    sort: string | null;
    icon: string | null;
    groupBy: string | null;
    parentId: number | null;
    sortOrder: number;
    count: number | null;
  }

  interface Tag {
    id: number;
    name: string;
    /** The group that gives it its colour. */
    groupId: number | null;
    color: string | null;
    count: number;
    starred: boolean;
    groupIds: number[];
  }

  interface TagGroup {
    id: number;
    name: string;
    color: string | null;
    icon: string | null;
    description: string | null;
  }

  interface Field {
    id: number;
    key: string;
    label: string;
    type: 'text' | 'integer' | 'float' | 'boolean' | 'date' | 'date_time' | 'single_select' | 'multi_select' | 'json' | 'url' | 'file_ref';
    description: string;
    defaultValue: string | null;
    options: string[];
    validation: { min?: number; max?: number; maxLength?: number; pattern?: string; required: boolean };
    format: string;
    widget: string;
    /** user, or the id of the plugin that defined it. */
    owner: string;
    visible: boolean;
    sortOrder: number;
    ownFilter: boolean;
    filterKind: string;
    /** Curio keeps what readers find for it, and it takes part in filters, sorts, columns and search. Always true for a field no reader fills. */
    indexed: boolean;
    /** The key of the field this one only makes sense with; switching one switches both. */
    pair: string | null;
    /** Number fields of one owner with the same name here are one filter of that name, with a range for each. */
    filterGroup: string | null;
    /** The readers that fill it from files; empty for a field filled by hand. */
    readers: string[];
    /** Switched on without reading its files yet. */
    needsScan: boolean;
  }

  interface FieldCoverage {
    key: string;
    /** Items with a value of it. */
    withValue: number;
    /** Files its readers that are on read. */
    applies: number;
    /** Files waiting to be read: new or changed, or queued for its readers. */
    waiting: number;
    /** Files one of its readers failed on. */
    failed: number;
  }

  interface FieldFacets {
    /** Each value with how many items have it. */
    values: [string | number, number][];
    set: number;
    total: number;
  }

  interface Facets {
    rating: Record<string, number>;
    shape: { landscape: number; portrait: number; square: number; wide: number; tall: number; '4:3': number; '16:9': number };
    kinds: Record<string, number>;
    exts: Record<string, number>;
    animatedExts: Record<string, number>;
    animated: number;
    total: number;
  }

  interface Location {
    id: number;
    name: string;
    online: boolean;
    watch: boolean;
    access: 'read_only' | 'full' | 'organize';
    count: number;
    lastScan: number | null;
    icon: string | null;
    color: string | null;
    path?: string;
  }

  /** What a window shows. */
  interface View {
    show: 'all' | 'favorites' | 'uncategorized' | 'untagged' | 'recent' | 'random' | 'trash' | 'missing' | 'folder' | 'smartFolder' | 'tag' | 'location' | 'locationFolder' | 'folders' | 'locations';
    /** The folder's, smart folder's, tag's or location's number; the numbers for several. */
    id: number | number[] | null;
    /** The folder inside a location. */
    path: string | null;
    name: string;
    search: string;
    /** The search and filters as one query. */
    query: string;
    count: number;
    /** tags: the tag manager; `tab:<plugin>/<tab>`: a Tab plugin's page. */
    view: 'grid' | 'tags' | `tab:${string}`;
    /** What font samples say, empty for each font's own line: view.setSample. */
    sample: { text: string };
  }

  interface Operation {
    name: string;
    description: string;
    permission: string;
    kind: 'read' | 'write' | 'window';
    /** JSON Schema. */
    params: object;
  }

  /** Long work a plugin does, shown in Curio's status area. */
  interface Job {
    id: string;
    /** The plugin it belongs to. */
    plugin: string | null;
    title: string;
    done: number;
    /** 0 when not known. */
    total: number;
    text: string;
    state: 'running' | 'done' | 'failed' | 'cancelled';
    /** What job.finish gave; for process.run `{ code, stdout, stderr, folder }`. */
    result: unknown;
    error: string | null;
    /** Milliseconds since 1970. */
    started: number;
    ended: number | null;
  }

  /** A program process.run started: at once, or with `wait` when it ended. */
  interface ProcessRun {
    job: string;
    /** The run's own folder, where the program starts. */
    folder: string;
    state: 'running' | 'done' | 'failed' | 'cancelled';
    /** With `wait`, once it ended: its exit code, the last megabyte it printed on each pipe, and what went wrong. */
    code?: number | null;
    stdout?: string;
    stderr?: string;
    error?: string | null;
  }

  /** A program Curio finds for plugins. */
  interface Tool {
    name: string;
    title: string;
    about: string;
    found: boolean;
    version: string | null;
    /** Where to get it. */
    homepage: string;
    /** Its place was chosen in Settings. */
    chosen: boolean;
    /** For callers that may run programs. */
    path?: string | null;
  }

  interface EventMap {
    /** A window shows another theme, or the same theme changed. `window` and what app.theme answers. Needs library.read. */
    'app.themeChanged': Theme & { window: string };
    /** The library list changed: a library added, removed, renamed, moved or given another look. `libraries`: the new list, as library.list gives it. Needs library.read. */
    'library.listChanged': { libraries: Library[] };
    /** A library opened in a window or in the background, or its background process showed its window. `name`, `state` (window or background). Needs library.read. */
    'library.opened': { name: string; state: 'window' | 'background' };
    /** No process holds the library any more. `name`. Needs library.read. */
    'library.closed': { name: string };
    /** New items, at most 1,000 to an event. `ids`, `how`: window, api, autoImport, scan, duplicate or restore; `requestId` of the API add that made them, where it had one. Needs metadata.read. */
    'items.added': { ids: string[]; how: 'window' | 'api' | 'autoImport' | 'scan' | 'duplicate' | 'restore'; requestId?: string };
    /** Items changed, at most 1,000 to an event. `ids`, `what`: metadata, file, thumbnail or palette. Needs metadata.read. */
    'items.changed': { ids: string[]; what: 'metadata' | 'file' | 'thumbnail' | 'palette' };
    /** Items moved to the trash, or deleted for good, at most 1,000 to an event. `ids`, `purged`. Needs metadata.read. */
    'items.removed': { ids: string[]; purged: boolean };
    /** Folders were made, renamed, moved, deleted or given another look; read folder.list again. Needs metadata.read. */
    'folders.changed': {};
    /** Smart folders changed; read smartFolder.list again. Needs metadata.read. */
    'smartFolders.changed': {};
    /** Tags were made, renamed, merged, deleted, coloured or grouped; read tag.list again. Needs metadata.read. */
    'tags.changed': {};
    /** Tag groups changed; read tagGroup.list again. Needs metadata.read. */
    'tagGroups.changed': {};
    /** Custom fields were defined, deleted, or switched on or off; read field.list again. Needs metadata.read. */
    'fields.changed': {};
    /** Locations were added, removed, changed or scanned, or went offline; read location.list again. Needs metadata.read. */
    'locations.changed': {};
    /** A location's scan under way: once as it starts, with `files` 0, then about once a second. `location`, `files` so far. Needs library.read. */
    'scan.progress': { location: number; files: number };
    /** A location's scan finished. `location`, `files`, `added`, `changed`, `moved`, `missing`, `offline`. Needs library.read. */
    'scan.done': { location: number; files: number; added: number; changed: number; moved: number; missing: number; offline: number };
    /** A window switched to another library. `window`, `from` and `to`: the libraries' ids. Needs library.read. */
    'library.switched': { window: string; from: string; to: string };
    /** The selection in a window changed. `window`, `count`, `ids`: the first 1000. Needs ui. */
    'selection.changed': { window: string; count: number; ids: string[] };
    /** A window shows something else: another folder, tag or view, another search or other filters, or its font samples another text. `window` and what view.get answers. Needs ui. */
    'view.changed': View & { window: string };
    /** A window's viewer shows an item, also when stepping to the next one or to another file grouped into it. `window`, `id`, `file`: the file shown, the item's own or one grouped into it. Needs ui. */
    'viewer.opened': { window: string; id: string; file: string };
    /** A plugin's job started: what job.get answers. Needs library.read. */
    'job.started': Job;
    /** A job got further. `job`, `plugin`, `done`, `total` (0 when not known), `text`. Needs library.read. */
    'job.progress': { job: string; plugin: string | null; done: number; total: number; text: string };
    /** Lines a job's program printed, gathered a few times a second. `job`, `plugin`, `pipe` (stdout or stderr), `lines`. Needs library.read. */
    'job.output': { job: string; plugin: string | null; pipe: 'stdout' | 'stderr'; lines: string[] };
    /** A job ended: done, failed or cancelled. What job.get answers. Needs library.read. */
    'job.finished': Job;
    /** A window's viewer closed. `window`, `id`: the item it showed last. Needs ui. */
    'viewer.closed': { window: string; id: string };
  }

  interface LibraryInfoParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryOpenParams {
    /** The library: its id from library.list, its folder or its name. */
    library: string;
  }

  interface LibraryCreateParams {
    /** The folder to make it in; Curio's own Libraries folder without one. */
    folder?: string;
    name: string;
    /** Open it in a window of its own once made. */
    open?: boolean;
  }

  interface LibraryRenameParams {
    /** The new name; the folder becomes `<name>.curio` beside the old one. */
    name: string;
    /** The library: its id from library.list, its folder or its name. */
    library: string;
  }

  interface LibraryForgetParams {
    /** The library: its id from library.list, its folder or its name. */
    library: string;
  }

  interface LibraryPlaceParams {
    /** The library it goes just before; last without one. */
    before?: string;
    /** The library: its id from library.list, its folder or its name. */
    library: string;
  }

  interface LibrarySetLookParams {
    /** A CSS colour; empty clears it. */
    color?: string;
    /** A Tabler icon name; empty clears it. */
    icon?: string;
    /** The library: its id from library.list, its folder or its name. */
    library: string;
  }

  interface ItemQueryParams {
    /** Newest, largest or highest first; true by default. */
    descending?: boolean;
    /** 50 by default, at most 1000. */
    limit?: number;
    offset?: number;
    /** Query language, e.g. `model = "Anima" AND steps <= 12`; empty for every item. */
    query?: string;
    /** all (default), uncategorized, untagged, recent, trash or missing. */
    scope?: string;
    /** imported (default), modified, created, name, size, rating, resolution, duration or field:<key>. */
    sort?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemCountParams {
    query?: string;
    /** all (default), uncategorized, untagged, recent, trash or missing. */
    scope?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemGetParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemReadFieldsParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** Only these fields; without them, the fields read from files that are switched off. */
    keys?: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemGetManyParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemUpdateParams {
    /** Notes. */
    annotation?: string;
    explicit?: boolean;
    favorite?: boolean;
    /** Custom field key to value; null clears a field. */
    fields?: Record<string, unknown>;
    /** One item's id. */
    id?: string | number;
    /** Several items' ids, all given the same values. */
    ids?: (string | number)[];
    name?: string;
    /** 0 to 5. */
    rating?: number;
    /** Where the item came from. */
    url?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRenameParams {
    items: ({ id: string | number; name: string })[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddTagsParams {
    ids: (string | number)[];
    /** Tag names. */
    tags: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRemoveTagsParams {
    /** Every tag the items have. */
    all?: boolean;
    ids: (string | number)[];
    /** Tags by name or number. */
    tags?: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddToFoldersParams {
    /** Folder ids. */
    folders: number[];
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRemoveFromFoldersParams {
    /** Folder ids. */
    folders: number[];
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemMoveToFolderParams {
    /** The folder they go to. */
    folder: number;
    /** The folder they leave, with its subfolders; without it they leave every folder they are in. */
    from?: number;
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemGroupParams {
    /** The item the files join. */
    id: string | number;
    /** The files that become part of it. */
    members: (string | number)[];
    /** What they are to it, such as preview or metadata; `part` unless set. */
    role?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemSplitParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemDetachParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemForgetFileParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAllowGroupingParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryGroupingParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryGroupingSurveyParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryGroupingApplyParams {
    /** Rule ids, as `library.grouping` lists them; every rule unless set. */
    rules?: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryGroupingSplitParams {
    /** A rule id, as `library.grouping` lists it. */
    rule: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemTrashParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRestoreParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemPurgeParams {
    /** Everything in the trash. */
    all?: boolean;
    /** Items in the trash. */
    ids?: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRotateParams {
    /** 0, 90, 180 or 270, clockwise. */
    degrees: number;
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRotateFileParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** Quarter turns clockwise, 1 by default. */
    turns?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemReplaceFileParams {
    /** The new file's contents as base64, or a data: address, for callers that have no file on this computer, such as plugin pages. */
    data?: string;
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** A file of the same type on this computer. */
    path?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemSetThumbnailParams {
    /** A picture as base64, or a data: address. */
    data?: string;
    ids: (string | number)[];
    /** A picture file on this computer. */
    path?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRefreshThumbnailParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRefreshPaletteParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemDuplicateParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemCopyToLibraryParams {
    ids: (string | number)[];
    /** The library they go to: its id from library.list, its folder or its name. */
    to: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemMoveToLibraryParams {
    ids: (string | number)[];
    /** The library they go to: its id from library.list, its folder or its name. */
    to: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemExportParams {
    /** A folder on this computer; made when missing. Files already there are kept, the copies get numbered names. */
    folder: string;
    ids: (string | number)[];
    /** The files grouped into each item go too; true unless set. */
    members?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemThumbnailParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemFileParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemExifParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemPngTextParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemPreviewUrlParams {
    /** The item's id. */
    id: string | number;
    /** The long edge wanted, in pixels; rounded up to a step of 256, from 256 to 8192. */
    size: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemFileUrlParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddFromPathsParams {
    /** Notes for the new items. */
    annotation?: string;
    /** Copy folders into the library too, instead of making them locations; files outside the locations are always copied. */
    copy?: boolean;
    /** Mark the new items as explicit. */
    explicit?: boolean;
    /** Mark the new items as favorites. */
    favorite?: boolean;
    /** Custom field values by key. */
    fields?: Record<string, unknown>;
    /** Folders to put the new items in. */
    folders?: number[];
    /** Take the files into the library instead of copying them, as a downloader does with its temporary files; files only, not folders. */
    move?: boolean;
    /** When a copied file's name is taken in its folder: keep both (default), replace or skip. */
    onClash?: string;
    /** Files or folders on this computer. */
    paths: string[];
    /** 0 to 5. */
    rating?: number;
    /** Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters. */
    requestId?: string;
    /** Where the new items came from, such as the page a video was downloaded from. */
    sourceUrl?: string;
    /** Tags for the new items; new names are created. */
    tags?: string[];
    /** Answer once the items are in, with their ids, waiting up to two minutes, instead of at once. */
    wait?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddFromUrlsParams {
    /** With one address in `urls`: other addresses of the same file, such as larger versions, tried in order before it; the first that answers with a file is kept. */
    alternates?: string[];
    /** Notes for the new items. */
    annotation?: string;
    /** `auto` (unless set): a file, except that an address that looks like a page is asked first and kept as a link unless it answers with a picture, video, sound, PDF or archive. `file` always downloads, `link` never does. */
    as?: string;
    /** A `Cookie` header for each download, as the browser sends it to that site: sent to the address's own host only, never to one a redirect leads to, and never stored. Not for plugins. */
    cookies?: string;
    /** Mark the new items as explicit. */
    explicit?: boolean;
    /** Mark the new items as favorites. */
    favorite?: boolean;
    /** Custom field values by key. */
    fields?: Record<string, unknown>;
    /** Folders to put the new items in. */
    folders?: number[];
    /** Hand addresses a plugin takes over (its `urls` in plugin.json) to that plugin; true unless set to false. A plugin's own addresses never come back to it. */
    handlers?: boolean;
    /** A name for the item, when only one address is given. */
    name?: string;
    /** A downloaded file whose bytes the library already holds, or a link to an address the library already has a link to: `keep` it (unless set), `skip` it, or keep it and `tell`; the answer's `duplicates` lists `{ id, of }`, or `{ of, skipped: true }`. */
    onDuplicate?: string;
    /** 0 to 5. */
    rating?: number;
    /** Sent as the `Referer` of each download, as the page that showed the file would; http and https only. */
    referer?: string;
    /** Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters. */
    requestId?: string;
    /** Where the new items came from, such as the page a video was downloaded from. */
    sourceUrl?: string;
    /** Tags for the new items; new names are created. */
    tags?: string[];
    /** Addresses of images, files or web pages. */
    urls: string[];
    /** The `User-Agent` of each download, such as the browser's own. */
    userAgent?: string;
    /** Wait for the downloads, up to a minute, and answer what they added (unless set to false); with false the answer is `queued` at once, and `items.added` tells of each item with this call's `requestId`. */
    wait?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddFromBytesParams {
    /** Notes for the new items. */
    annotation?: string;
    /** The file as base64, or a data: address; up to 64 MB of request. */
    data?: string;
    /** Mark the new items as explicit. */
    explicit?: boolean;
    /** Mark the new items as favorites. */
    favorite?: boolean;
    /** Custom field values by key. */
    fields?: Record<string, unknown>;
    /** Folders to put the new items in. */
    folders?: number[];
    /** The file's name with its extension, e.g. `sunset.jpg`. */
    name: string;
    /** A file whose bytes the library already holds: `keep` it (unless set), `skip` it, or keep it and `tell`; the answer's `duplicates` lists `{ id, of }`, or `{ of, skipped: true }`. */
    onDuplicate?: string;
    /** 0 to 5. */
    rating?: number;
    /** Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters. */
    requestId?: string;
    /** Where the new items came from, such as the page a video was downloaded from. */
    sourceUrl?: string;
    /** Tags for the new items; new names are created. */
    tags?: string[];
    /** In place of `data`: the id `POST /api/v1/upload` answered, for files up to 2 GB. It works once, for the token that sent it, within ten minutes. */
    upload?: string;
    /** Where the file came from. */
    url?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddBookmarkParams {
    /** Notes for the new items. */
    annotation?: string;
    /** Mark the new items as explicit. */
    explicit?: boolean;
    /** Mark the new items as favorites. */
    favorite?: boolean;
    /** Custom field values by key. */
    fields?: Record<string, unknown>;
    /** Folders to put the new items in. */
    folders?: number[];
    /** The link's name; the address without one. */
    name?: string;
    /** 0 to 5. */
    rating?: number;
    /** Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters. */
    requestId?: string;
    /** Where the new items came from, such as the page a video was downloaded from. */
    sourceUrl?: string;
    /** Tags for the new items; new names are created. */
    tags?: string[];
    url: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemAddBesideParams {
    /** The file as base64, or a data: address. */
    data?: string;
    /** The item the new one goes beside. */
    id: string | number;
    /** The new file's name with its extension; the item's own name with the new file's extension without it. */
    name?: string;
    /** A file on this computer. */
    path?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemWriteUrlParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** `replace` (the default) or `beside`. */
    mode?: string;
    /** For `beside`: the new file's name with its extension. */
    name?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginStateGetParams {
    key: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginStateSetParams {
    key: string;
    /** Any JSON; null removes the key. */
    value: unknown;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginItemStateGetParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginItemStateGetManyParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginItemStateSetParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** Any JSON; null removes it. */
    value: unknown;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface PluginSettingsSetParams {
    /** The settings to change; null removes one. */
    values: Record<string, unknown>;
  }

  interface PluginLogParams {
    /** `info` (the default), `warn` or `error`. */
    level?: string;
    message: string;
  }

  interface JobStartParams {
    /** Shown in Curio's status area. */
    title: string;
    /** How much there is to do, in any unit; 0 or none when it isn't known. */
    total?: number;
  }

  interface JobUpdateParams {
    done?: number;
    job: string;
    /** A line on what it is doing now. */
    text?: string;
    total?: number;
  }

  interface JobFinishParams {
    /** What went wrong; the job is then failed. */
    error?: string;
    job: string;
    /** What the job made, kept for job.get for ten minutes. */
    result?: unknown;
  }

  interface JobCancelParams {
    job: string;
  }

  interface JobGetParams {
    job: string;
  }

  interface ProcessRunParams {
    /** The tool's arguments. */
    args?: string[];
    /** A program and its arguments; a first element naming a file in the plugin's folder runs that file (`["bin/convert", "--fast"]`), anything else is looked for on the system's path. */
    program?: string[];
    /** Text given to the program on its input. */
    stdin?: string;
    /** Seconds before the program is stopped: an hour unless set, at most a day. */
    timeout?: number;
    /** Shown in Curio's status area; the program's name without it. */
    title?: string;
    /** A tool Curio finds, which the plugin lists in its `tools`: ffmpeg, ffprobe, yt-dlp, gallery-dl, exiftool, magick or 7z. */
    tool?: string;
    /** Answer when the program ends, waiting up to two minutes, with its exit code and output. */
    wait?: boolean;
  }

  interface ArchiveListParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ArchiveEntryParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** The entry's name, as archive.list gives it. */
    name: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ArchiveEntryUrlParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** The entry's name, as archive.list gives it. */
    name: string;
    /** For a picture: its long edge in pixels, scaled down by Curio; the entry as it is without it. */
    size?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ArchiveCreateParams {
    /** Notes for the new items. */
    annotation?: string;
    /** Mark the new items as explicit. */
    explicit?: boolean;
    /** Mark the new items as favorites. */
    favorite?: boolean;
    /** Custom field values by key. */
    fields?: Record<string, unknown>;
    /** Folders to put the new items in. */
    folders?: number[];
    /** The items whose files go in, in this order: the pages of a comic. */
    items: (string | number)[];
    /** The new item's file name; `.cbz` is added unless it ends with `.cbz` or `.zip`. */
    name: string;
    /** 0 to 5. */
    rating?: number;
    /** Names this add: a second call with the same id within ten minutes answers what the first did (`queued` while it runs), with `repeat: true`, and adds nothing. Up to 128 characters. */
    requestId?: string;
    /** Where the new items came from, such as the page a video was downloaded from. */
    sourceUrl?: string;
    /** Tags for the new items; new names are created. */
    tags?: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemSimilarParams {
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** 50 unless set, at most 1000. */
    limit?: number;
    /** How alike, 0 to 1: the share of the fingerprint's bits that agree. 0.8 unless set. */
    threshold?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface DuplicatesFindParams {
    /** Only the items in this folder and its subfolders. */
    folder?: number;
    /** Only these items. */
    ids?: (string | number)[];
    /** The kinds whose thumbnails `looks` compares; every kind but video and audio, which have ways of their own, unless set. */
    lookKinds?: string[];
    /** Files smaller than this many bytes are left out. */
    minSize?: number;
    /** Only the items this query finds. */
    query?: string;
    /** How alike, 0 to 1, by way: `{ looks: 0.9, video: 0.9, sound: 0.7, archive: 0.9 }` unless set. */
    thresholds?: Record<string, number>;
    /** The ways to compare: `content` (identical files), `looks` (thumbnails that look alike: pictures, covers, first pages), `video` (frames through each video), `sound` (the first minute of each sound) and `archive` (the entries of ZIP, CBZ, EPUB and 7z files); `content` unless set. */
    ways?: string[];
    /** Compare those items with every item in the library, not only with each other: copies of them anywhere. */
    withLibrary?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface DuplicatesResolveParams {
    /** The kept item takes its copies' tags, folders, best rating, notes, source address and user fields; true unless set. */
    combine?: boolean;
    /** Up to 1000 groups. */
    groups: ({ identical?: (string | number)[]; keep: string | number; remove: (string | number)[] })[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface DuplicatesIgnoreParams {
    /** Two or more items, none of which is a duplicate of another. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface DuplicatesUnignoreParams {
    /** The items whose marks go; every mark unless set. */
    ids?: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LibraryStatsParams {
    /** kind, ext, folder, tag or month. */
    by: string;
    /** Only the items this query finds; every item unless set. */
    query?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderGetParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderRecentParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderItemsParams {
    descending?: boolean;
    id: number;
    /** 50 by default, at most 1000. */
    limit?: number;
    offset?: number;
    /** imported (default), modified, created, name, size, rating, resolution, duration or field:<key>. */
    sort?: string;
    /** Include the subfolders' items; true by default. */
    subfolders?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderCreateParams {
    name: string;
    /** The folder to make it in; top level without one. */
    parent?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderRenameParams {
    id: number;
    name: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderMoveParams {
    id: number;
    /** The new parent folder; null or missing for the top level. */
    parent?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderUpdateParams {
    /** Tags given to items added to the folder. */
    autoTags?: string[];
    /** A CSS colour; empty clears it. */
    color?: string;
    /** A Tabler icon name; empty clears it. */
    icon?: string;
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FolderDeleteParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderGetParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderCreateParams {
    /** Virtual subfolders grouped by a property or field (e.g. `year:added`, `rating`, `field:genre`). */
    groupBy?: string;
    name: string;
    /** A smart folder to narrow. */
    parentId?: number;
    query: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderUpdateParams {
    /** Virtual subfolders grouped by a property or field; empty clears it. */
    groupBy?: string;
    /** A Tabler icon name; empty clears it. */
    icon?: string;
    id: number;
    name?: string;
    query?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderDeleteParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface SmartFolderItemsParams {
    id: number;
    /** 50 by default, at most 1000. */
    limit?: number;
    offset?: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGetParams {
    /** The tag's name, or its number. */
    tag: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagRenameParams {
    newName: string;
    /** The tag's name, or its number. */
    tag: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagMergeParams {
    /** Tags to merge into `target`. */
    sources: (string | number)[];
    /** The tag they become; made when it doesn't exist. */
    target: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagDeleteParams {
    /** The tag's name, or its number. */
    tag: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagSetColorParams {
    /** A CSS colour; null or empty clears it. */
    color?: string;
    /** The tag's name, or its number. */
    tag: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupCreateParams {
    color?: string;
    name: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupUpdateParams {
    /** A CSS colour; empty clears it. */
    color?: string;
    description?: string;
    /** A Tabler icon name; empty clears it. */
    icon?: string;
    id: number;
    name?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupDeleteParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupAddTagsParams {
    groupId: number;
    /** Tags by name or number; new names are created. */
    tags: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface TagGroupRemoveTagsParams {
    groupId: number;
    /** Tags by name or number. */
    tags: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldGetParams {
    key: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldDefineParams {
    defaultValue?: string;
    description?: string;
    /** Number fields of one owner with the same name here are one filter of that name, with a range for each; empty takes the field out. */
    filterGroup?: string;
    /** How that filter asks: list, range, text or has; empty picks by type. */
    filterKind?: string;
    format?: string;
    /** Curio keeps what readers find for it, and it takes part in filters, sorts, columns and search; its pair follows. */
    indexed?: boolean;
    /** Lower case, used in queries: `steps >= 20`. */
    key: string;
    label?: string;
    /** Switched on without reading the files yet; `field.rescan` reads them. */
    needsScan?: boolean;
    /** The choices of a select or multiselect field. */
    options?: string[];
    /** A filter of its own in the filter bar and smart folder rules. */
    ownFilter?: boolean;
    /** The key of the field this one only makes sense with, as `lora_strength` with `lora`. */
    pair?: string;
    sortOrder?: number;
    /** text, integer, float, boolean, date, datetime, single_select, multi_select, json, url or file_ref; needed for a new field. */
    type?: string;
    /** `{ min, max, required }`. */
    validation?: unknown;
    visible?: boolean;
    /** row (default), block or code. */
    widget?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldDeleteParams {
    key: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldFacetsParams {
    key: string;
    /** Among the items this query finds; every item without one. */
    query?: string;
    /** all (default), uncategorized, untagged, recent, trash or missing. */
    scope?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldCoverageParams {
    /** Only these fields; all of them without it. */
    keys?: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldFailuresParams {
    keys?: string[];
    /** 200 unless given; at most 2000. */
    limit?: number;
    /** That plugin's readers. */
    plugin?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldUsedByParams {
    key: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldCleanupParams {
    /** Count only. */
    dryRun?: boolean;
    /** Only these fields. A field whose plugin is gone, which can't be read again, is cleaned only when named here. */
    keys?: string[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface FieldRescanParams {
    /** Only these items; without them, every file the readers apply to. */
    items?: (string | number)[];
    /** The fields to read: every reader that fills one of them reads its files. */
    keys?: string[];
    /** missing: only files with none of the readers' fields, filling only what has no value; all (the default): what was read before is replaced, and what a file no longer has is cleared. Values set by hand stay either way. */
    mode?: string;
    /** Read with this plugin's readers. */
    plugin?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationListParams {
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationAddParams {
    /** What Curio may do to its files: read_only, full or organize, set before its first scan. Full by default. */
    access?: string;
    /** Its name in the sidebar; the folder's own name by default. */
    label?: string;
    /** A folder on this computer. */
    path: string;
    /** Follow changes to its files as they happen; true by default. */
    watch?: boolean;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationRemoveParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationRescanParams {
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationSetAccessParams {
    /** read_only, full or organize. */
    access: string;
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationSetLookParams {
    /** A CSS colour; empty clears it. */
    color?: string;
    /** A Tabler icon name; empty clears it. */
    icon?: string;
    id: number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface LocationRelocateParams {
    id: number;
    /** Where the folder is now. */
    path: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface QueryValidateParams {
    query?: string;
    /** all (default), uncategorized, untagged, recent, trash or missing. */
    scope?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface QueryFacetsParams {
    query?: string;
    /** all (default), uncategorized, untagged, recent, trash or missing. */
    scope?: string;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ViewerSetToolsParams {
    buttons?: ({ icon?: string; id: string; label: string; on?: boolean; panel?: unknown[] })[];
    /** The ids of the plugin's `viewer_keys` that do something now; all of them when left out. */
    keys?: string[];
    /** Text labels shown in the toolbar, such as a page number or chapter title. */
    labels?: ({ id: string; text: string })[];
    /** A slider for the place in what is shown. */
    slider?: { dir?: string; id: string; label?: string; max: number; min: number; step?: number; value: number };
  }

  interface PluginCloseParams {
    /** The answer of the popover the page shows in. */
    result?: unknown;
  }

  interface AppThemeParams {
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface LibrarySwitchParams {
    /** The library to show: its id from library.list, its folder or its name. */
    to: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface SelectionGetParams {
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface SelectionSetParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewGetParams {
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewShowParams {
    /** The folder's, smart folder's or location's number, or the tag's name or number. */
    id?: string | number;
    /** all, favorites, uncategorized, untagged, recent, random, trash, missing, folder, smartFolder, tag or location. */
    what: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewSetSearchParams {
    /** Words, or a query such as `rating >= 4 AND tag = sky`; empty clears the search. */
    text: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewSetSampleParams {
    /** What the samples say; empty for the line each font's own script has. Left out, it stays as it is. */
    text?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewSetFiltersParams {
    /** Items added on this day (YYYY-MM-DD) or later. */
    addedFrom?: string;
    /** Items added on this day (YYYY-MM-DD) or earlier. */
    addedTo?: string;
    /** Items added within this many days. */
    addedWithin?: number;
    /** A colour as #rrggbb, or grayscale. */
    color?: string;
    /** Items whose file was created on this day (YYYY-MM-DD) or later. */
    createdFrom?: string;
    /** Items whose file was created on this day (YYYY-MM-DD) or earlier. */
    createdTo?: string;
    /** True for explicit items only, false to exclude explicit items. */
    explicit?: boolean;
    /** File extensions such as png or mp4. */
    exts?: string[];
    /** True for favorites only, false to exclude favorites. */
    favorite?: boolean;
    /** Items in any of these folders, by number. */
    folders?: number[];
    /** Items in none of these folders. */
    foldersNot?: number[];
    /** image, video, audio, document, code, link, archive, font, model or other. */
    kinds?: string[];
    /** Items whose file was modified on this day (YYYY-MM-DD) or later. */
    modifiedFrom?: string;
    /** Items whose file was modified on this day (YYYY-MM-DD) or earlier. */
    modifiedTo?: string;
    /** Star ratings, 0 to 5. */
    ratings?: number[];
    /** landscape, portrait, square, wide, tall, 4:3 or 16:9. */
    shapes?: string[];
    /** How `tags` combine: any (the default), all, or exactly (these tags and no others). */
    tagRule?: string;
    /** Items with these tags, by name or number. */
    tags?: (string | number)[];
    /** Items without any of these tags. */
    tagsNot?: (string | number)[];
    /** Items with no tags at all, besides those with `tags`. */
    untagged?: boolean;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewShowWithParams {
    /** A field's key, with `value`. */
    field?: string;
    /** A tag's name. */
    tag?: string;
    /** The field's value: text, or a number for a number field. */
    value?: unknown;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewerOpenParams {
    /** The item's id. */
    id: string | number;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewerCloseParams {
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ViewerCurrentParams {
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ItemOpenParams {
    /** viewer (the default): in the window's viewer; window: in a window of its own; app: in the app the system opens its type with. */
    how?: string;
    /** An item's stable id, or its row number inside this library. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface ItemRevealParams {
    /** The item's id. */
    id: string | number;
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  interface AppOpenUrlParams {
    url: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiNotifyParams {
    /** info (the default) or error. */
    kind?: string;
    text: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiConfirmParams {
    body?: string;
    /** The confirming button's label; OK by default. */
    okLabel?: string;
    title: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiPromptParams {
    body?: string;
    okLabel?: string;
    placeholder?: string;
    title: string;
    /** The text the box starts with. */
    value?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiPickTagsParams {
    /** Tags chosen to begin with, by name or number; names that aren't tags yet are left out. */
    selected?: (string | number)[];
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiPickFoldersParams {
    /** Folders chosen to begin with, by number. */
    selected?: number[];
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiPickFieldParams {
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiPickColorParams {
    title?: string;
    /** The colour to begin with, as #rrggbb. */
    value?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiMenuParams {
    items: ({ disabled?: boolean; icon?: string; id?: string; items?: ({ disabled?: boolean; icon?: string; id?: string; label?: string; separator?: boolean })[]; label?: string; separator?: boolean })[];
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface UiChooseParams {
    /** A field's box in the page, in CSS pixels from the page's top left. */
    at?: { h: number; w: number; x: number; y: number };
    /** Values, or `{ value, label, icon, hint, heading, disabled }`: `hint` is faint text at the row's end, `heading` a title over the rows after it; up to 1000. */
    options: unknown[];
    /** A search field at the list's top. */
    search?: boolean;
    /** The option ticked as the current one. */
    value?: unknown;
  }

  interface UiPopoverParams {
    /** A field's box in the page, in CSS pixels from the page's top left. */
    at?: { h: number; w: number; x: number; y: number };
    /** Told to that page as `plugin.run`. */
    data?: unknown;
    /** A page in the plugin's folder. */
    page: string;
    /** 160 to 640 pixels; 300 when left out. */
    width?: number;
  }

  interface DialogOpenFileParams {
    filters?: ({ extensions: string[]; name: string })[];
    /** Several files at once. */
    multiple?: boolean;
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface DialogOpenFolderParams {
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface DialogSaveFileParams {
    /** The name the dialog suggests. */
    fileName?: string;
    filters?: ({ extensions: string[]; name: string })[];
    title?: string;
    /** The window showing this library: the library's id from library.list, its folder or its name. */
    library?: string;
    /** The window: its id from library.list. Without it or `library`, the most recently focused window. */
    window?: string;
  }

  interface ClipboardWriteTextParams {
    text: string;
  }

  interface ClipboardCopyFilesParams {
    /** Item ids. */
    ids: (string | number)[];
    /** The library to use: its id from library.list, its folder or its name. Without it, the library of the most recently used window. */
    library?: string;
  }

  /** Every operation, by namespace. */
  interface Api {
    app: {
      /** Curio's version, the API version, the display language, and the system it runs on: `platform` as `windows`, `linux` or `macos`, `arch` as `x86_64` or `aarch64`. Needs library.read. */
      info(): Promise<{ name: string; version: string; apiVersion: number; platform: 'windows' | 'linux' | 'macos'; arch: string; language: string }>;
      /** The operations this caller may use, with their parameters. Needs library.read. */
      operations(): Promise<Operation[]>;
      /** The font families installed on this computer, for a reader's choice of font. Needs library.read. */
      fonts(): Promise<string[]>;
      /** The theme a window shows: its name, whether it is dark, and every colour, spacing and font token as CSS variables take them. Needs library.read. Takes window or library. */
      theme(params?: AppThemeParams): Promise<Theme>;
      /** Open a web address in the system's browser, once the person at the window agrees to it; http, https and mailto only. Needs ui. Takes window or library. */
      openUrl(params: AppOpenUrlParams): Promise<{ opened: boolean }>;
    };
    token: {
      /** Delete the token this call was made with, as a client does when it disconnects; it works for nothing after this answer. Needs library.read. */
      forget(): Promise<Done>;
    };
    library: {
      /** The libraries on this computer's list, with their ids and whether a window has them open. Needs library.read. */
      list(): Promise<Library[]>;
      /** A library's id, name and item counts. Needs library.read. Takes library. */
      info(params?: LibraryInfoParams): Promise<LibraryInfo>;
      /** Open a library in a window of its own; a window that already has it comes forward. Needs library.manage. Names the library with library. */
      open(params: LibraryOpenParams): Promise<Done>;
      /** Make a new library and add it to the list. Needs library.manage. */
      create(params: LibraryCreateParams): Promise<{ success: true; id: string; name: string; path?: string }>;
      /** Rename a library's folder, which is its name; only while no window has it open. Needs library.manage. Names the library with library. */
      rename(params: LibraryRenameParams): Promise<{ success: true; name: string; path?: string }>;
      /** Take a library off the list; its folder stays as it is. Needs library.manage. Names the library with library. */
      forget(params: LibraryForgetParams): Promise<Done>;
      /** Move a library in the list. Needs library.manage. Names the library with library. */
      place(params: LibraryPlaceParams): Promise<Done>;
      /** Set a library's icon or colour in the lists. Needs library.manage. Names the library with library. */
      setLook(params: LibrarySetLookParams): Promise<Done>;
      /** The rules grouping files into one item, in the order they are tried, each with how many files it grouped. Needs metadata.read. Takes library. */
      grouping(params?: LibraryGroupingParams): Promise<{ rules: { id: string; owner: string; role: string; grouped: number; exts?: string[]; files?: string[] }[] }>;
      /** How many items and bytes there are by kind, extension, folder (with its subfolders), tag or month added, among the items a query finds. Needs metadata.read. Takes library. */
      stats(params: LibraryStatsParams): Promise<{ by: string; total: { count: number; bytes: number }; groups: { key: string; name: string; count: number; bytes: number }[] }>;
      /** Show another library in a window; if another window has it, that window comes forward instead. Needs library.manage. Takes window or library. */
      switch(params: LibrarySwitchParams): Promise<{ switched: boolean; window: string }>;
      grouping: {
        /** How many files already in the library each rule would group; nothing is grouped. Needs metadata.read. Takes library. */
        survey(params?: LibraryGroupingSurveyParams): Promise<{ rules: { id: string; count: number }[] }>;
        /** Group the files already in the library as the rules say, or only these `rules`; when that takes over two minutes the answer is `queued` and the work carries on. Needs metadata.write. Takes library. */
        apply(params?: LibraryGroupingApplyParams): Promise<{ count: number } | { queued: true }>;
        /** Split off every file a rule grouped, each an item again with what it had; what other rules grouped stays, and rules may group these files again; when that takes over two minutes the answer is `queued` and the work carries on. Needs metadata.write. Takes library. */
        split(params: LibraryGroupingSplitParams): Promise<{ count: number } | { queued: true }>;
      };
    };
    item: {
      /** Find items with the query language, e.g. `tag = sky AND rating >= 4` or `prompt CONTAINS "blue hair"`. Needs metadata.read. Takes library. */
      query(params?: ItemQueryParams): Promise<Page<Item>>;
      /** Count the items a query finds. Needs metadata.read. Takes library. */
      count(params?: ItemCountParams): Promise<{ count: number }>;
      /** One item with its custom fields. Needs metadata.read. Takes library. */
      get(params: ItemGetParams): Promise<ItemWithFields>;
      /** Reads an item's file for fields read from files without storing what it finds: by default the fields switched off, which hold no values. Curio's readers and plugins' declared ones run; plugins' programs don't. Needs files.read. Takes library. */
      readFields(params: ItemReadFieldsParams): Promise<{ fields: Record<string, unknown> }>;
      /** Several items with their custom fields. Needs metadata.read. Takes library. */
      getMany(params: ItemGetManyParams): Promise<ItemWithFields[]>;
      /** Change items' names, notes, source addresses, ratings or custom fields. Needs metadata.write. Takes library. */
      update(params?: ItemUpdateParams): Promise<Done>;
      /** Give several items names of their own in one change. Needs metadata.write. Takes library. */
      rename(params: ItemRenameParams): Promise<Done>;
      /** Add tags to items; new tag names are created. Needs metadata.write. Takes library. */
      addTags(params: ItemAddTagsParams): Promise<Done>;
      /** Remove tags from items, or all their tags. Needs metadata.write. Takes library. */
      removeTags(params: ItemRemoveTagsParams): Promise<Done>;
      /** Add items to folders. Needs metadata.write. Takes library. */
      addToFolders(params: ItemAddToFoldersParams): Promise<Done>;
      /** Take items out of folders; they stay in the library. Needs metadata.write. Takes library. */
      removeFromFolders(params: ItemRemoveFromFoldersParams): Promise<Done>;
      /** Put items in a folder and take them out of another, or out of every other folder. Needs metadata.write. Takes library. */
      moveToFolder(params: ItemMoveToFolderParams): Promise<Done>;
      /** Group files into one item: `members` become files of `id`, which takes their tags, folders and notes; splitting gives them back. Needs metadata.write. Takes library. */
      group(params: ItemGroupParams): Promise<Done>;
      /** Split items into their files, each an item again with what it had; rules don't group them again. Needs metadata.write. Takes library. */
      split(params: ItemSplitParams): Promise<{ ids: string[] }>;
      /** Take one file out of the item it is grouped into, an item again. Needs metadata.write. Takes library. */
      detach(params: ItemDetachParams): Promise<Done>;
      /** Forget a missing or offline file grouped into an item: it leaves the item and the library, and a scan doesn't bring it back unchanged. Needs files.write. Takes library. */
      forgetFile(params: ItemForgetFileParams): Promise<Done>;
      /** Let rules group items split by hand again. Needs metadata.write. Takes library. */
      allowGrouping(params: ItemAllowGroupingParams): Promise<Done>;
      /** Move items to the trash; files on disk stay until the trash is emptied. Needs metadata.write. Takes library. */
      trash(params: ItemTrashParams): Promise<Done>;
      /** Bring items back from the trash, into the folders they were in. Needs metadata.write. Takes library. */
      restore(params: ItemRestoreParams): Promise<Done>;
      /** Delete items in the trash for good, or empty the trash; copies the library made go to the Recycle Bin. Needs files.write. Takes library. */
      purge(params?: ItemPurgeParams): Promise<Done>;
      /** Turn pictures as they are shown; the files stay as they are. Needs metadata.write. Takes library. */
      rotate(params: ItemRotateParams): Promise<Done>;
      /** Turn a picture's file itself by quarter turns clockwise (JPG, PNG, BMP, still WebP). Needs files.write. Takes library. */
      rotateFile(params: ItemRotateFileParams): Promise<Done>;
      /** Put another file of the same type in an item's place; the old file goes to the Recycle Bin. A `path` needs files.outside, unless a run of the plugin's own made it. Needs files.write. Takes library. */
      replaceFile(params: ItemReplaceFileParams): Promise<Done>;
      /** Give items a thumbnail from a picture file or picture bytes. A `path` needs files.outside, unless a run of the plugin's own made it. Needs files.write. Takes library. */
      setThumbnail(params: ItemSetThumbnailParams): Promise<Done>;
      /** Make items' thumbnails again. Needs metadata.write. Takes library. */
      refreshThumbnail(params: ItemRefreshThumbnailParams): Promise<Done>;
      /** Read items' colours again from their thumbnails. Needs metadata.write. Takes library. */
      refreshPalette(params: ItemRefreshPaletteParams): Promise<{ success: true; queued?: true }>;
      /** New items with their own copies of the files, the same metadata and folders. Needs files.write. Takes library. */
      duplicate(params: ItemDuplicateParams): Promise<{ success: true; ids: string[] }>;
      /** Copy items to another library, open or not, with their tags, rating, notes and matching fields. Needs files.write. Takes library. */
      copyToLibrary(params: ItemCopyToLibraryParams): Promise<{ success: true; count: number }>;
      /** Copy items to another library, then move them to this library's trash. Needs files.write. Takes library. */
      moveToLibrary(params: ItemMoveToLibraryParams): Promise<{ success: true; count: number }>;
      /** Copy items' files into a folder on this computer. Needs files.outside. Takes library. */
      export(params: ItemExportParams): Promise<{ success: true; count: number }>;
      /** An item's thumbnail; over HTTP the answer is the image itself. Needs files.read. Takes library. */
      thumbnail(params: ItemThumbnailParams): Promise<Blob>;
      /** An item's file; over HTTP the answer is the file itself. Needs files.read. Takes library. */
      file(params: ItemFileParams): Promise<Blob>;
      /** The EXIF tags of an item's picture, by name: text as text, other values as EXIF shows them; empty without EXIF. Needs files.read. Takes library. */
      exif(params: ItemExifParams): Promise<{ tags: Record<string, string> }>;
      /** The text chunks of an item's PNG file (tEXt, zTXt, iTXt), by keyword; empty for other files. Needs files.read. Takes library. */
      pngText(params: ItemPngTextParams): Promise<{ chunks: Record<string, string> }>;
      /** An address a plugin's page shows an item's picture from at `size` pixels on its long edge, made once by Curio and kept, as its own viewer does when zooming: JPEGs are read at the reduced size, so a large page never decodes in full. Works for every format Curio can picture, camera raw and PSD included. Needs files.read. Plugin pages only. */
      previewUrl(params: ItemPreviewUrlParams): Promise<{ url: string }>;
      /** An address a plugin's page reads an item's file from, in parts as it needs (for `<video>`, pdf.js or a reader); for the formats the plugin declares, files beside it resolve too, as an OBJ's materials or a glTF's textures. Needs files.read. Plugin pages only. */
      fileUrl(params: ItemFileUrlParams): Promise<{ url: string }>;
      /** An address a plugin's page sends a file to with PUT, once: `replace` puts it in the item's place (same type; the old file goes to the Recycle Bin), `beside` makes it a new item next to the item, as item.add.beside. The PUT answers as those calls do. Needs files.write. Plugin pages only. */
      writeUrl(params: ItemWriteUrlParams): Promise<{ url: string }>;
      /** Pictures that look like an item, most alike first, from the fingerprint Curio keeps of every thumbnail. Needs metadata.read. Takes library. */
      similar(params: ItemSimilarParams): Promise<{ id: string; similarity: number }[]>;
      /** Open an item in its library's window, in a window of its own, or in the app the system opens its type with; programs and scripts open only from Curio's own window, saved links only when they point to a web page. Needs ui. Takes library. */
      open(params: ItemOpenParams): Promise<Done>;
      /** Show an item's file in the system's file manager. Needs ui. Takes library. */
      reveal(params: ItemRevealParams): Promise<Done>;
      add: {
        /** Add files or folders from this computer, copied into the library, a folder as a library folder with its subfolders; without `copy` a folder becomes a location instead, and a file already in a location is that location's item. Needs files.outside too, unless a run of the plugin's own made the files. Needs files.write. Takes library. */
        fromPaths(params: ItemAddFromPathsParams): Promise<{ success: true; queued: true } | { success: true; ids: string[] }>;
        /** Download images and files into the library; web pages and videos are saved as links. Curio downloads only from the internet, not from this computer or its network. Needs files.write. Takes library. */
        fromUrls(params: ItemAddFromUrlsParams): Promise<Added>;
        /** Add a file sent in the call, as base64. Needs files.write. Takes library. */
        fromBytes(params: ItemAddFromBytesParams): Promise<Added>;
        /** Save a web address as a link, whatever it points to. Needs files.write. Takes library. */
        bookmark(params: ItemAddBookmarkParams): Promise<Added>;
        /** Add a file as a new item next to an item, in its folders and with its tags, rating, notes, source address and fields: a converted or edited copy. A `path` needs files.outside, unless a run of the plugin's own made it. Needs files.write. Takes library. */
        beside(params: ItemAddBesideParams): Promise<Added>;
      };
    };
    plugin: {
      /** Write a line to Curio's log, marked with the plugin's id. Needs library.read. Plugins only: their pages and programs. */
      log(params: PluginLogParams): Promise<Done>;
      /** Close the plugin's window the page is in, or the popover it shows in (`ui.popover`) with `result` as that popover's answer. Answered by that page's window. Needs library.read. Plugin pages only. */
      close(params?: PluginCloseParams): Promise<Done>;
      state: {
        /** A value the plugin keeps for this library under a key; null when none is. Needs library.read. Plugins only: their pages and programs. */
        get(params: PluginStateGetParams): Promise<{ value: unknown }>;
        /** Keep any JSON value for this library under a key, up to 1 MB; null removes it. Needs library.read. Plugins only: their pages and programs. */
        set(params: PluginStateSetParams): Promise<Done>;
      };
      itemState: {
        /** What the plugin keeps for one item, such as a reading position; null when nothing is. Needs library.read. Plugins only: their pages and programs. */
        get(params: PluginItemStateGetParams): Promise<{ value: unknown }>;
        /** What the plugin keeps for several items, by id; items with nothing kept are left out. Needs library.read. Plugins only: their pages and programs. */
        getMany(params: PluginItemStateGetManyParams): Promise<Record<string, unknown>>;
        /** Keep a value for one item, up to 64 KB, removed with the item; null removes it. Unlike a field nobody sees or searches it, and no items.changed is sent, so saving a place every few seconds wakes no window. Needs library.read. Plugins only: their pages and programs. */
        set(params: PluginItemStateSetParams): Promise<Done>;
      };
      settings: {
        /** The plugin's settings on this computer, the same in every library. Needs library.read. Plugins only: their pages and programs. */
        get(): Promise<Record<string, unknown>>;
        /** Change the plugin's settings on this computer: each key given is set, null removes it. The plugin's pages in the same window hear plugin.settingsChanged. Needs library.read. Plugins only: their pages and programs. */
        set(params: PluginSettingsSetParams): Promise<Record<string, unknown>>;
      };
    };
    job: {
      /** Start a job: long work the plugin or one of Curio's tools does itself, shown in Curio's status area with its progress and a way to cancel it, so a window can be closed while it runs. Report with job.update and end it with job.finish. Needs library.read. Plugins only: their pages and programs. */
      start(params: JobStartParams): Promise<{ job: string }>;
      /** Tell how far a job has got; the answer says whether it was cancelled meanwhile, so the work can stop. Needs library.read. Plugins only: their pages and programs. */
      update(params: JobUpdateParams): Promise<{ cancelled: boolean }>;
      /** End a job with its result, or as failed with `error`. Needs library.read. Plugins only: their pages and programs. */
      finish(params: JobFinishParams): Promise<Done>;
      /** Cancel a running job: a program process.run started is stopped with everything it started; a plugin doing the work itself hears job.finished. Needs library.read. Plugins only: their pages and programs. */
      cancel(params: JobCancelParams): Promise<Done>;
      /** Running jobs and those that ended in the last ten minutes; a plugin gets its own. Needs library.read. */
      list(): Promise<Job[]>;
      /** One job: its progress, and once it ends its result or error. Needs library.read. */
      get(params: JobGetParams): Promise<Job>;
    };
    process: {
      /** Run one of the plugin's programs, or a tool it lists in `tools`, as a job: its output comes as job.output events, cancelling stops it and everything it started, and it works in a folder of its own in Curio's temporary space. Answers at once with the job, or with `wait` when it ends. Needs exec. Plugins only: their pages and programs. */
      run(params?: ProcessRunParams): Promise<ProcessRun>;
    };
    archive: {
      /** What a ZIP, CBZ, EPUB, 7z or CB7 item holds, in reading order (folder by folder, `page2` before `page10`), each entry with its size and whether it is a picture; pictures with the size they show at and whether they move, read from their first bytes (in a 7z, once it has been unpacked for reading). Needs files.read. Takes library. */
      list(params: ArchiveListParams): Promise<{ entries: { name: string; size: number; picture: boolean; width?: number; height?: number; animated?: boolean }[] }>;
      /** One entry of an archive item; over HTTP the answer is the entry itself. Needs files.read. Takes library. */
      entry(params: ArchiveEntryParams): Promise<Blob>;
      /** An address a plugin's page shows an archive's entry from, as `<img>` takes it; with `size` a picture comes scaled to that long edge, as item.previewUrl does. Needs files.read. Plugin pages only. */
      entryUrl(params: ArchiveEntryUrlParams): Promise<{ url: string }>;
      /** Pack items' files into a new CBZ (or ZIP) item; 7z is read, not written, in the order given: pictures become a comic's pages. Needs files.write. Takes library. */
      create(params: ArchiveCreateParams): Promise<Added>;
    };
    duplicates: {
      /** Look for duplicates as a job: identical files, and with other `ways` pictures, videos, sounds and archives that look or sound alike; the hashes and fingerprints it works out are kept for the next search. The job's result is the groups, each member matching every other one, and what couldn't be compared; job.finished tells when it is there. A search already running in the library stops. Needs metadata.read. Takes library. */
      find(params?: DuplicatesFindParams): Promise<{ job: string }>;
      /** Keep one item of each group and move the others to the trash, the kept one first taking their tags, folders, best rating, notes, source address and user fields it hasn't any of. Each group is checked first: one whose kept item is gone, or a copy said to be identical that changed, is left as it is. Needs metadata.write. Takes library. */
      resolve(params: DuplicatesResolveParams): Promise<{ removed: number; kept: number; skipped: { id: string; reason: string }[] }>;
      /** Mark items as not duplicates of each other: no search groups any two of them again. Needs metadata.write. Takes library. */
      ignore(params: DuplicatesIgnoreParams): Promise<{ pairs: number }>;
      /** Take back marks of not being duplicates: those of some items, or all of them. Needs metadata.write. Takes library. */
      unignore(params?: DuplicatesUnignoreParams): Promise<{ pairs: number }>;
    };
    tool: {
      /** The programs Curio finds for plugins (FFmpeg, yt-dlp …): whether each is found, its version, and where to get it. Needs library.read. */
      list(): Promise<Tool[]>;
    };
    folder: {
      /** Every folder as a flat list with parent ids and item counts. Needs metadata.read. Takes library. */
      list(params?: FolderListParams): Promise<Folder[]>;
      /** One folder with the ids of its subfolders. Needs metadata.read. Takes library. */
      get(params: FolderGetParams): Promise<Folder & { children: number[] }>;
      /** The folders used last in this library's window, newest first. Needs metadata.read. Takes library. */
      recent(params?: FolderRecentParams): Promise<Folder[]>;
      /** The items in a folder, its subfolders' too unless asked otherwise. Needs metadata.read. Takes library. */
      items(params: FolderItemsParams): Promise<Page<Item>>;
      /** Make a folder. Needs organize. Takes library. */
      create(params: FolderCreateParams): Promise<{ success: true; id: number }>;
      /** Rename a folder. Needs organize. Takes library. */
      rename(params: FolderRenameParams): Promise<Done>;
      /** Move a folder under another, or to the top level. Needs organize. Takes library. */
      move(params: FolderMoveParams): Promise<Done>;
      /** Set a folder's colour, icon or automatic tags. Needs organize. Takes library. */
      update(params: FolderUpdateParams): Promise<Done>;
      /** Delete a folder and its subfolders; their items stay in the library. Needs organize. Takes library. */
      delete(params: FolderDeleteParams): Promise<Done>;
    };
    smartFolder: {
      /** Every smart folder with its query. Needs metadata.read. Takes library. */
      list(params?: SmartFolderListParams): Promise<SmartFolder[]>;
      /** One smart folder. Needs metadata.read. Takes library. */
      get(params: SmartFolderGetParams): Promise<SmartFolder>;
      /** Make a smart folder from a query; inside another one it narrows that one. Needs organize. Takes library. */
      create(params: SmartFolderCreateParams): Promise<{ success: true; id: number }>;
      /** Rename a smart folder, change its query or icon. Needs organize. Takes library. */
      update(params: SmartFolderUpdateParams): Promise<Done>;
      /** Delete a smart folder. Needs organize. Takes library. */
      delete(params: SmartFolderDeleteParams): Promise<Done>;
      /** The items a smart folder shows. Needs metadata.read. Takes library. */
      items(params: SmartFolderItemsParams): Promise<Page<Item>>;
    };
    tag: {
      /** Every tag with its item count and groups. Needs metadata.read. Takes library. */
      list(params?: TagListParams): Promise<Tag[]>;
      /** One tag with its item count, colour and groups. Needs metadata.read. Takes library. */
      get(params: TagGetParams): Promise<Tag>;
      /** Rename a tag; renaming onto an existing tag merges them. Needs organize. Takes library. */
      rename(params: TagRenameParams): Promise<Done>;
      /** Merge several tags into one. Needs organize. Takes library. */
      merge(params: TagMergeParams): Promise<Done>;
      /** Delete a tag from every item. Needs organize. Takes library. */
      delete(params: TagDeleteParams): Promise<Done>;
      /** Set or clear a tag's own colour. Needs organize. Takes library. */
      setColor(params: TagSetColorParams): Promise<Done>;
    };
    tagGroup: {
      /** Every tag group. Needs metadata.read. Takes library. */
      list(params?: TagGroupListParams): Promise<TagGroup[]>;
      /** Make a tag group. Needs organize. Takes library. */
      create(params: TagGroupCreateParams): Promise<{ success: true; id: number }>;
      /** Rename a tag group or set its colour, icon or description. Needs organize. Takes library. */
      update(params: TagGroupUpdateParams): Promise<Done>;
      /** Delete a tag group; its tags stay. Needs organize. Takes library. */
      delete(params: TagGroupDeleteParams): Promise<Done>;
      /** Put tags into a group; new tag names are created. Needs organize. Takes library. */
      addTags(params: TagGroupAddTagsParams): Promise<Done>;
      /** Take tags out of a group. Needs organize. Takes library. */
      removeTags(params: TagGroupRemoveTagsParams): Promise<Done>;
    };
    field: {
      /** Custom metadata fields: key, label, type and owner. Needs metadata.read. Takes library. */
      list(params?: FieldListParams): Promise<Field[]>;
      /** One custom field. Needs metadata.read. Takes library. */
      get(params: FieldGetParams): Promise<Field>;
      /** Define a custom field, or change one; properties left out keep their values. Needs organize. Takes library. */
      define(params: FieldDefineParams): Promise<Done>;
      /** Delete a field defined by the user, with its values. Needs organize. Takes library. */
      delete(params: FieldDeleteParams): Promise<Done>;
      /** A field's values with how many items have each, among the items a query finds. Needs metadata.read. Takes library. */
      facets(params: FieldFacetsParams): Promise<FieldFacets>;
      /** How far the fields read from files are read: for each, the items with a value, the files its readers apply to, those waiting to be read and those a reader failed on. Needs metadata.read. Takes library. */
      coverage(params?: FieldCoverageParams): Promise<{ fields: FieldCoverage[] }>;
      /** The files readers failed on, newest first: for some fields or a plugin's readers, or all. Needs metadata.read. Takes library. */
      failures(params?: FieldFailuresParams): Promise<{ failures: { id: string; name: string; reader: string; error: string; at: number }[] }>;
      /** What names a field: smart folders, and plugins' filters, panels and sorts. Needs metadata.read. Takes library. */
      usedBy(params: FieldUsedByParams): Promise<{ smartFolders: { id: number; name: string }[]; filters: { plugin: string; label: string }[]; sorts: { plugin: string; label: string }[] }>;
      /** Remove the stored values of fields read from files that are switched off, all but those set by hand; with dryRun, only count them. Definitions and what names the fields stay. Needs metadata.write. Takes library. */
      cleanup(params?: FieldCleanupParams): Promise<{ fields: { key: string; values: number; pluginGone: boolean }[]; removed: number }>;
      /** Read fields from files again: those of some fields or of a plugin's readers, for some items or every file they apply to; answers the job that follows the reading. Needs metadata.write. Takes library. */
      rescan(params?: FieldRescanParams): Promise<{ job: string | null; files: number }>;
    };
    location: {
      /** The folders on this computer the library indexes in place. Needs metadata.read. Takes library. */
      list(params?: LocationListParams): Promise<Location[]>;
      /** Index a folder in place: its files join the library where they are. An Eagle library (a `.library` folder) is indexed as its items, with its folders and tags, and the other options don't apply to it. Needs files.outside. Takes library. */
      add(params: LocationAddParams): Promise<{ success: true; id: number }>;
      /** Stop indexing a folder; its items leave the library, its files stay. Needs organize. Takes library. */
      remove(params: LocationRemoveParams): Promise<Done>;
      /** Look through a location again for new, changed and missing files. Needs organize. Takes library. */
      rescan(params: LocationRescanParams): Promise<{ success: true; queued: true }>;
      /** What Curio may do to a location's files: read_only, full (edit files) or organize (edit and move them). Needs organize. Takes library. */
      setAccess(params: LocationSetAccessParams): Promise<Done>;
      /** Set a location's icon or colour. Needs organize. Takes library. */
      setLook(params: LocationSetLookParams): Promise<Done>;
      /** Point a location at the folder's new place after it was moved. Needs organize. Takes library. */
      relocate(params: LocationRelocateParams): Promise<Done>;
    };
    query: {
      /** Check a query: whether it parses, and how many items it finds. Needs metadata.read. Takes library. */
      validate(params?: QueryValidateParams): Promise<{ valid: true; count: number } | { valid: false; error: string }>;
      /** Ratings, shapes and kinds with their counts among the items a query finds. Needs metadata.read. Takes library. */
      facets(params?: QueryFacetsParams): Promise<Facets>;
    };
    viewer: {
      /** A viewer page's texts, slider and buttons in Curio's own toolbar, each drawn by the widget its plugin declares under the same id (`viewer_tools`); a click comes to the page as `plugin.tool` with the button's id, a right-click with `menu: true` as well, and a value set with the slider or a control in a button's panel with `value`. Answered by the page showing the plugin; an empty list clears them. Needs library.read. Plugin pages only. */
      setTools(params?: ViewerSetToolsParams): Promise<Done>;
      /** Open an item in a window's viewer; a window that doesn't show it switches to all items. A file grouped into an item opens that item, showing the file when it was grouped in by hand. An item whose file is missing or whose storage is offline is refused (`failed`). Needs ui. Takes window or library. */
      open(params: ViewerOpenParams): Promise<Done>;
      /** Close a window's viewer. Needs ui. Takes window or library. */
      close(params?: ViewerCloseParams): Promise<{ closed: boolean }>;
      /** The item open in a window's viewer, and `file`, the file it shows: the item's own or one grouped into it. Needs ui. Takes window or library. */
      current(params?: ViewerCurrentParams): Promise<{ id: string | null; file: string | null }>;
    };
    selection: {
      /** The items selected in a window. Needs ui. Takes window or library. */
      get(params?: SelectionGetParams): Promise<Item[]>;
      /** Select items in a window; items its grid doesn't show are left out, unless it shows a Tab plugin's page. Needs ui. Takes window or library. */
      set(params: SelectionSetParams): Promise<{ selected: number; notShown: number }>;
    };
    view: {
      /** What a window shows: the folder, tag or view, its search and filters as a query, and how many items. Needs ui. Takes window or library. */
      get(params?: ViewGetParams): Promise<View>;
      /** Show a folder, smart folder, tag, location, or all, uncategorized, untagged, recent, random, trash or missing items in a window; its search and filters are cleared unless its filter lock is on. Needs ui. Takes window or library. */
      show(params: ViewShowParams): Promise<View>;
      /** Search in a window: words, or a query such as `rating >= 4`. Needs ui. Takes window or library. */
      setSearch(params: ViewSetSearchParams): Promise<View>;
      /** The text a window's font samples are written in (the list's Font sample column, the Fonts viewer); an empty text goes back to their own. Needs ui. Takes window or library. */
      setSample(params?: ViewSetSampleParams): Promise<View>;
      /** Set a window's filters: tags, folders, ratings, kinds, extensions, colour, shapes, how recently items were added and the days they were added, modified or created. Needs ui. Takes window or library. */
      setFilters(params?: ViewSetFiltersParams): Promise<View>;
      /** Show all items in a window through one filter, as a click on a tag or a field's value does in Curio's own widgets: a tag, or a field's value set as that field's filter asks (the value, a range of just it, or the text it contains); the filter chip shows it, to be widened or taken off. Needs ui. Takes window or library. */
      showWith(params?: ViewShowWithParams): Promise<View>;
    };
    ui: {
      /** Show a short message in a window. Needs ui. Takes window or library. */
      notify(params: UiNotifyParams): Promise<Done>;
      /** Ask the person at a window to confirm something. Needs ui. Takes window or library. */
      confirm(params: UiConfirmParams): Promise<{ ok: boolean }>;
      /** Ask the person at a window for a line of text. Needs ui. Takes window or library. */
      prompt(params: UiPromptParams): Promise<{ value: string | null }>;
      /** Let the person at a window choose tags. Needs ui. Takes window or library. */
      pickTags(params?: UiPickTagsParams): Promise<{ tags: string[] | null }>;
      /** Let the person at a window choose folders. Needs ui. Takes window or library. */
      pickFolders(params?: UiPickFoldersParams): Promise<{ folders: number[] | null }>;
      /** Let the person at a window choose a custom field. Needs ui. Takes window or library. */
      pickField(params?: UiPickFieldParams): Promise<{ key: string | null }>;
      /** Let the person at a window choose a colour. Needs ui. Takes window or library. */
      pickColor(params?: UiPickColorParams): Promise<{ color: string | null }>;
      /** Show a menu at the pointer in a window and answer the entry chosen. Needs ui. Takes window or library. */
      menu(params: UiMenuParams): Promise<{ id: string | null }>;
      /** A list of options under one of the page's fields, as Curio's menus are, drawn by the window so the page's frame doesn't cut it off; `at` is the field's box in the page. Answered by the window showing the page. Needs library.read. Plugin pages only. */
      choose(params: UiChooseParams): Promise<{ chosen: boolean; value?: string | number | boolean | null }>;
      /** Another page of the plugin in a popover under one of the page's fields, drawn by the window; that page hears `data` as `plugin.run` and closes it with `plugin.close({ result })`. Answered by the window showing the page. Needs library.read. Plugin pages only. */
      popover(params: UiPopoverParams): Promise<{ result: unknown }>;
    };
    dialog: {
      /** Let the person at a window choose files with the system's dialog. Needs ui. Takes window or library. */
      openFile(params?: DialogOpenFileParams): Promise<{ paths: string[] | null }>;
      /** Let the person at a window choose a folder with the system's dialog. Needs ui. Takes window or library. */
      openFolder(params?: DialogOpenFolderParams): Promise<{ path: string | null }>;
      /** Let the person at a window choose where to save a file with the system's dialog; nothing is written. Needs ui. Takes window or library. */
      saveFile(params?: DialogSaveFileParams): Promise<{ path: string | null }>;
    };
    clipboard: {
      /** The text on the clipboard. Needs ui. */
      readText(): Promise<{ text: string | null }>;
      /** Put text on the clipboard. Needs ui. */
      writeText(params: ClipboardWriteTextParams): Promise<Done>;
      /** Put items' files on the clipboard, to paste in a file manager. Needs ui. Takes library. */
      copyFiles(params: ClipboardCopyFilesParams): Promise<{ count: number }>;
    };
  }
}

declare const Curio: {
  connect(options: Curio.Options): Curio.Client;
  Error: { new (code: string, message: string, status?: number): Curio.CurioError };
};
