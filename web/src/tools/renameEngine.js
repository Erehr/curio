// Batch rename's methods and tags, without the page: a setup in, every item's new name out.
// Methods run top to bottom on the name without its extension, each on what the one before left.

export const METHODS = ['newName', 'add', 'replace', 'list', 'remove', 'case', 'trim']

export const DEFAULTS = {
  newName: { on: true, pattern: '<Name>' },
  add: { on: false, text: '', where: 'end', at: 1, fromEnd: false, find: '', matchCase: false, regex: false },
  replace: { on: false, find: '', with: '', which: 'all', matchCase: false, regex: false },
  list: { on: false, pairs: [{ find: '', with: '' }], matchCase: false, regex: false },
  remove: { on: false, type: 'text', text: '', matchCase: false, regex: false, start: 1, count: 1, fromEnd: false, chars: '' },
  trim: { on: false, chars: ' ._-', where: 'both' },
  case: { on: false, name: 'title', keep: '', ext: 'keep' },
}

/** A setup with every method and option, what is missing taken from the defaults. */
export function complete(setup = {}) {
  const out = {}
  for (const m of METHODS) out[m] = { ...DEFAULTS[m], ...(setup?.[m] || {}) }
  out.list.pairs = Array.isArray(out.list.pairs) && out.list.pairs.length ? out.list.pairs.map((p) => ({ find: String(p?.find ?? ''), with: String(p?.with ?? '') })) : [{ find: '', with: '' }]
  return out
}

/** Tags of New name and Replace with, as the tag list shows them: [tag, id, group]; what each gives is the window's text, `tools.rename.tags.<id>`. */
export const TAGS = [
  ['<Name>', 'name', 'name'],
  ['<Ext>', 'ext', 'name'],
  ['<Word:1>', 'word', 'name'],
  ['<Substr:1:5>', 'substr', 'name'],
  ['<Inc Nr>', 'incNr', 'number'],
  ['<Inc NrDir>', 'incNrDir', 'number'],
  ['<Inc Alpha>', 'incAlpha', 'number'],
  ['<Num Items>', 'numItems', 'number'],
  ['<Num Files>', 'numFiles', 'number'],
  ['<FolderName>', 'folderName', 'item'],
  ['<Folders>', 'folders', 'item'],
  ['<Tags>', 'tags', 'item'],
  ['<Kind>', 'kind', 'item'],
  ['<Width>', 'width', 'item'],
  ['<Height>', 'height', 'item'],
  ['<Rating>', 'rating', 'item'],
  ['<Field:key>', 'field', 'item'],
  ['<Date>', 'date', 'date'],
  ['<Time>', 'time', 'date'],
  ['<Added>', 'added', 'date'],
  ['<Modified>', 'modified', 'date'],
]

const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const pad = (v, arg) => {
  const s = String(v)
  const w = /^0+$/.test(arg || '') ? arg.length : Number(arg) || 0
  return s.padStart(w, '0')
}
const two = (n) => String(n).padStart(2, '0')

/** A date in a form of yyyy, yy, mm, m, dd, d, hh, h, nn, n, ss, s, other characters as they are. */
export function formatDate(d, form) {
  const parts = { yyyy: d.getFullYear(), yy: two(d.getFullYear() % 100), mm: two(d.getMonth() + 1), m: d.getMonth() + 1, dd: two(d.getDate()), d: d.getDate(), hh: two(d.getHours()), h: d.getHours(), nn: two(d.getMinutes()), n: d.getMinutes(), ss: two(d.getSeconds()), s: d.getSeconds() }
  return form.replace(/yyyy|yy|mm|m|dd|d|hh|h|nn|n|ss|s/gi, (t) => String(parts[t.toLowerCase()]))
}

/** Letters counting as a spreadsheet's columns: 0 is a, 25 z, 26 aa. */
function letters(n, upper) {
  let s = ''
  for (n += 1; n > 0; n = Math.floor((n - 1) / 26)) s = String.fromCharCode(97 + ((n - 1) % 26)) + s
  return upper ? s.toUpperCase() : s
}
const fromLetters = (s) => [...s.toLowerCase()].reduce((n, c) => n * 26 + (c.charCodeAt(0) - 96), 0) - 1

const words = (s) => s.split(/[\s_.\-]+/).filter(Boolean)
const nth = (list, i) => list[i < 0 ? list.length + i : i - 1]

/** A tag's value for one item, or null for a tag that is not one, which then stays as it is written. */
function tagValue(name, args, it, x) {
  switch (name) {
    case 'name': return x.stem
    case 'ext': return it.ext || ''
    case 'kind': return it.kind || ''
    case 'word': {
      const list = words(x.stem)
      const from = Number(args[0] || 1)
      const count = Number(args[1] || 1)
      const i = from < 0 ? list.length + from : from - 1
      return i < 0 ? '' : list.slice(i, i + count).join(' ')
    }
    case 'substr': {
      const from = Number(args[0] || 1)
      const start = from < 0 ? Math.max(0, x.stem.length + from) : Math.max(0, from - 1)
      return args[1] ? x.stem.substr(start, Number(args[1])) : x.stem.slice(start)
    }
    case 'inc nr':
    case 'inc nrdir': {
      const n = name === 'inc nr' ? x.index : x.inFolder
      const first = args[0] && /^\d+$/.test(args[0]) ? Number(args[0]) : 1
      return pad(first + n * (Number(args[1]) || 1), args[0]?.startsWith('0') ? '0'.repeat(args[0].length) : '')
    }
    case 'inc alpha': {
      const start = /^[a-z]+$/i.test(args[0] || '') ? args[0] : 'a'
      return letters(fromLetters(start) + x.index * (Number(args[1]) || 1), start === start.toUpperCase())
    }
    case 'num items': return pad(x.total, args[0])
    case 'num files': return pad(x.folderTotal, args[0])
    case 'foldername': return it.folders?.[0]?.name || ''
    case 'folders': return (it.folders || []).map((f) => f.name).join(' ')
    case 'tags': return (it.tags || []).join(' ')
    case 'width': return it.width == null ? '' : pad(it.width, args[0])
    case 'height': return it.height == null ? '' : pad(it.height, args[0])
    case 'rating': return it.rating ? String(it.rating) : ''
    case 'field': {
      const v = it.fields?.[args.join(':')]
      return v == null ? '' : Array.isArray(v) ? v.join(' ') : String(v)
    }
    case 'date': return formatDate(x.now, args.join(':') || 'yyyy-mm-dd')
    case 'time': return formatDate(x.now, args.join(':') || 'hh-nn-ss')
    case 'added': return it.imported ? formatDate(new Date(it.imported), args.join(':') || 'yyyy-mm-dd') : ''
    case 'modified': return it.modified ? formatDate(new Date(it.modified), args.join(':') || 'yyyy-mm-dd') : ''
    default: return null
  }
}

/** Text with its tags filled in for one item, in one pass so a value is never read as a tag; `quote` changes what a value holds before it goes in. */
export function fillTags(text, it, x, quote = (s) => s) {
  return text.replace(/<([a-z][a-z ]*?)((?::[^<>]*)?)>/gi, (whole, name, rest) => {
    const v = tagValue(name.toLowerCase().replace(/\s+/g, ' ').trim(), rest ? rest.slice(1).split(':') : [], it, x)
    return v == null ? whole : quote(v)
  })
}

/** A `$1`, `$<name>`, `$&` or `$$` replacement for one match, as String.replace reads it. */
function fillGroups(rep, m) {
  return rep.replace(/\$(\$|&|`|'|\d{1,2}|<([^>]*)>)/g, (whole, t, named) => {
    if (t === '$') return '$'
    if (t === '&') return m[0]
    if (t === '`') return m.input.slice(0, m.index)
    if (t === "'") return m.input.slice(m.index + m[0].length)
    if (named !== undefined) return m.groups?.[named] ?? ''
    const n = Number(t)
    return n && n < m.length ? (m[n] ?? '') : whole
  })
}

/** A search as a global expression: a regular expression as written, or text where `*` stands for any run of characters when `wild` is set. */
function search(text, { regex, matchCase }, wild = false) {
  if (!text) return null
  const flags = matchCase ? 'gu' : 'giu'
  if (regex) return new RegExp(text, flags)
  return new RegExp(wild ? text.split('*').map(escapeRe).join('.*?') : escapeRe(text), flags)
}

/** Replaces every match, or the first or the last; `rep` is a function of the match. */
function replaceIn(s, re, rep, which = 'all') {
  const found = [...s.matchAll(re)]
  const pick = which === 'first' ? found.slice(0, 1) : which === 'last' ? found.slice(-1) : found
  let out = ''
  let at = 0
  for (const m of pick) {
    out += s.slice(at, m.index) + rep(m)
    at = m.index + m[0].length
  }
  return out + s.slice(at)
}

function changeCase(s, how, keep) {
  let out
  if (how === 'lower') out = s.toLowerCase()
  else if (how === 'upper') out = s.toUpperCase()
  else if (how === 'invert') out = [...s].map((c) => (c === c.toUpperCase() ? c.toLowerCase() : c.toUpperCase())).join('')
  else if (how === 'sentence') out = s.toLowerCase().replace(/\p{L}/u, (c) => c.toUpperCase())
  else if (how === 'title') out = s.toLowerCase().replace(/(^|[^\p{L}\p{N}'’])(\p{L})/gu, (_, a, b) => a + b.toUpperCase())
  else return s
  // words the user wants written as they are, as DVD or of
  for (const w of keep) out = out.replace(new RegExp(`(?<![\\p{L}\\p{N}])${escapeRe(w)}(?![\\p{L}\\p{N}])`, 'giu'), w)
  return out
}

/** Each method's problem, as a regular expression that is not one, by method; a method with one is skipped. */
function problems(s) {
  const bad = {}
  const tryRe = (m, text, on) => {
    if (!on || !text) return
    try {
      new RegExp(text, 'u')
    } catch (e) {
      bad[m] = e.message
    }
  }
  tryRe('add', s.add.find, s.add.on && s.add.regex && (s.add.where === 'before' || s.add.where === 'after'))
  tryRe('replace', s.replace.find, s.replace.on && s.replace.regex)
  for (const p of s.list.pairs) tryRe('list', p.find, s.list.on && s.list.regex)
  tryRe('remove', s.remove.text, s.remove.on && s.remove.regex && s.remove.type === 'text')
  return bad
}

/**
 * Plans a rename: `bad` names the methods that cannot run, `names(items, now)` gives each item's new name without its extension and its extension.
 * Numbers count over the items in their order; per folder, by each item's first folder, the items in none counting together.
 */
export function plan(setup) {
  const s = complete(setup)
  const bad = problems(s)
  const on = (m) => s[m].on && !bad[m]
  const re = {
    add: on('add') && (s.add.where === 'before' || s.add.where === 'after') ? search(s.add.find, s.add) : null,
    replace: on('replace') ? search(s.replace.find, s.replace) : null,
    list: on('list') ? s.list.pairs.filter((p) => p.find).map((p) => [search(p.find, s.list), p.with]) : [],
    remove: on('remove') && s.remove.type === 'text' ? search(s.remove.text, s.remove, !s.remove.regex) : null,
  }
  const keep = s.case.keep.split(',').map((w) => w.trim()).filter(Boolean)
  const trimSet = s.trim.chars ? `[${escapeRe(s.trim.chars)}]+` : null
  const trimRe = trimSet && new RegExp(s.trim.where === 'start' ? `^${trimSet}` : s.trim.where === 'end' ? `${trimSet}$` : `^${trimSet}|${trimSet}$`, 'gu')
  const charsRe = s.remove.chars ? new RegExp(`[${escapeRe(s.remove.chars)}]`, s.remove.matchCase ? 'gu' : 'giu') : null

  function one(it, x) {
    let n = x.stem
    // an empty pattern leaves the name, as a method switched off does
    if (on('newName') && s.newName.pattern.trim()) n = fillTags(s.newName.pattern, it, x)
    if (on('add') && s.add.text) {
      const a = s.add
      const t = fillTags(a.text, it, x)
      const chars = [...n]
      if (a.where === 'start') n = t + n
      else if (a.where === 'end') n = n + t
      else if (a.where === 'position') {
        // after this many characters, from the start or the end
        const k = Math.min(chars.length, Math.max(0, Number(a.at) || 0))
        const i = a.fromEnd ? chars.length - k : k
        n = chars.slice(0, i).join('') + t + chars.slice(i).join('')
      } else if (re.add) {
        re.add.lastIndex = 0
        const m = re.add.exec(n)
        if (m) {
          const i = a.where === 'before' ? m.index : m.index + m[0].length
          n = n.slice(0, i) + t + n.slice(i)
        }
      }
    }
    if (re.replace) {
      const rep = fillTags(s.replace.with, it, x, s.replace.regex ? (v) => v.replace(/\$/g, '$$$$') : undefined)
      n = replaceIn(n, re.replace, s.replace.regex ? (m) => fillGroups(rep, m) : () => rep, s.replace.which)
    }
    for (const [r, w] of re.list) n = replaceIn(n, r, s.list.regex ? (m) => fillGroups(w, m) : () => w)
    if (on('remove')) {
      const r = s.remove
      if (re.remove) n = replaceIn(n, re.remove, () => '')
      else if (r.type === 'position') {
        const chars = [...n]
        const count = Math.max(0, Number(r.count) || 0)
        const at = Math.max(0, (Number(r.start) || 1) - 1)
        if (r.fromEnd) chars.splice(Math.max(0, chars.length - at - count), Math.min(count, Math.max(0, chars.length - at)))
        else chars.splice(at, count)
        n = chars.join('')
      } else if (r.type === 'chars' && charsRe) n = n.replace(charsRe, '')
      else if (r.type === 'digits') n = n.replace(/\p{N}/gu, '')
      else if (r.type === 'symbols') n = n.replace(/[^\p{L}\p{N}\s]/gu, '')
      else if (r.type === 'accents') n = n.normalize('NFD').replace(/\p{M}/gu, '').normalize('NFC')
      else if (r.type === 'brackets') n = n.replace(/\s*(\([^()]*\)|\[[^[\]]*\]|\{[^{}]*\})/g, '')
    }
    let ext = x.extText
    if (on('case')) {
      n = changeCase(n, s.case.name, keep)
      ext = s.case.ext === 'lower' ? ext.toLowerCase() : s.case.ext === 'upper' ? ext.toUpperCase() : ext
    }
    if (on('trim') && trimRe) n = n.replace(trimRe, '')
    return { stem: n.replace(/\s+/g, ' ').trim(), ext }
  }

  function names(items, now = new Date()) {
    const totals = new Map()
    const key = (it) => (it.folders?.length ? it.folders[0].id : 'none')
    for (const it of items) totals.set(key(it), (totals.get(key(it)) || 0) + 1)
    const seen = new Map()
    return items.map((it, index) => {
      const k = key(it)
      const inFolder = seen.get(k) || 0
      seen.set(k, inFolder + 1)
      return one(it, { index, inFolder, total: items.length, folderTotal: totals.get(k), now, stem: stemOf(it), extText: extOf(it) })
    })
  }
  return { bad, names }
}

export const stemOf = (it) => (it.ext && it.name.toLowerCase().endsWith('.' + it.ext.toLowerCase()) ? it.name.slice(0, -it.ext.length - 1) : it.name)
export const extOf = (it) => (it.ext && it.name.toLowerCase().endsWith('.' + it.ext.toLowerCase()) ? it.name.slice(-it.ext.length - 1) : '')
