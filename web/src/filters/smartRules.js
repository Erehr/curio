// Smart folder rules: the builder's rows, turned into a search query and back again.
// A saved smart folder is only its query, so the builder has to be able to read one.

import { t } from '../shared/i18n.js'

// shapes and kinds by id; their names are `filters.shapes.<id>` and `filters.kinds.<id>` in the catalogs
export const SHAPES = ['landscape', 'portrait', 'square', 'wide', 'tall']
export const KINDS = ['image', 'video', 'audio', 'document', 'code', 'link', 'archive', 'font', 'model', 'other']
export const SIZE_UNITS = ['KB', 'MB', 'GB']
export const DURATION_UNITS = ['sec', 'min', 'hour']
export const AGE_UNITS = ['days', 'weeks', 'years']
const UNIT_SUFFIX = { KB: 'kb', MB: 'mb', GB: 'gb', sec: 's', min: 'm', hour: 'h', days: 'd', weeks: 'w', years: 'y' }

// words looked for everywhere, as the search box does with every place ticked
export const TEXT_PROP = ['__text', null, 'words']

export const BUILTIN_PROPS = [
  TEXT_PROP, ['name', null, 'text'], ['tag', null, 'tags'], ['folder', null, 'folders'], ['ext', null, 'text'], ['kind', null, 'kind'],
  ['rating', null, 'rating'], ['color', null, 'color'], ['shape', null, 'shape'],
  ['width', null, 'num'], ['height', null, 'num'], ['aspect', null, 'num'], ['animated', null, 'yesno'], ['size', null, 'size'], ['duration', null, 'duration'], ['pages', null, 'num'],
  ['added', null, 'date'], ['created', null, 'date'], ['modified', null, 'date'],
  ['annotation', null, 'text'], ['url', null, 'text'], ['tag_count', null, 'num'], ['folder_color', null, 'foldercolor'],
  ['part', null, 'select', ['motion', 'raw', 'jpeg', 'edits', 'metadata', 'preview']],
]

// each type's operators: the query's and the id of its words, `filters.ops.<type>.<id>` in the catalogs
export const OPS = {
  text: [['CONTAINS', 'contains'], ['NOT CONTAINS', 'doesNotContain'], ['=', 'is'], ['!=', 'isNot'], ['STARTS WITH', 'startsWith'], ['ENDS WITH', 'endsWith'], ['MATCHES', 'matchesRegex'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  tags: [['IN', 'anyOf'], ['HAS ALL', 'allOf'], ['EXACTLY', 'exactly'], ['HAS NONE', 'noneOf'], ['CONTAINS', 'hasOneWith'], ['NOT CONTAINS', 'hasNoneWith'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  folders: [['IN', 'anyOf'], ['HAS ALL', 'allOf'], ['HAS NONE', 'noneOf'], ['CONTAINS', 'hasOneWith'], ['NOT CONTAINS', 'hasNoneWith'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  num: [['=', 'is'], ['!=', 'isNot'], ['>=', 'atLeast'], ['<=', 'atMost'], ['>', 'moreThan'], ['<', 'lessThan'], ['BETWEEN', 'between'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  size: [['>=', 'atLeast'], ['<=', 'atMost'], ['>', 'moreThan'], ['<', 'lessThan'], ['BETWEEN', 'between']],
  duration: [['>=', 'atLeast'], ['<=', 'atMost'], ['>', 'moreThan'], ['<', 'lessThan'], ['BETWEEN', 'between'], ['IS EMPTY', 'none'], ['IS NOT EMPTY', 'hasOne']],
  rating: [['=', 'is'], ['!=', 'isNot'], ['>=', 'atLeast'], ['<=', 'atMost']],
  color: [['~', 'similar'], ['=', 'accurate']],
  shape: [['=', 'is'], ['!=', 'isNot']],
  kind: [['=', 'is'], ['!=', 'isNot']],
  date: [['WITHIN', 'inTheLast'], ['>', 'after'], ['>=', 'onOrAfter'], ['<', 'before'], ['<=', 'onOrBefore'], ['=', 'on'], ['BETWEEN', 'between']],
  bool: [['=', 'is']],
  select: [['=', 'is'], ['!=', 'isNot'], ['CONTAINS', 'contains'], ['NOT CONTAINS', 'doesNotContain'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  words: [['CONTAINS', 'contains'], ['NOT CONTAINS', 'doesNotContain']],
  yesno: [['=', 'is']],
  foldercolor: [['=', 'is'], ['!=', 'isNot'], ['IS EMPTY', 'isEmpty'], ['IS NOT EMPTY', 'isNotEmpty']],
  query: [['=', 'matches']],
}

// the operators of a Tags or Folders row that take a list of names, chosen in the window's pickers
export const LIST_OPS = new Set(['IN', 'HAS ALL', 'EXACTLY', 'HAS NONE'])
export const opLabel = (type, o) => t(`filters.ops.${type}.${o[1]}`)
export const isListRule = (r) => LIST_OPS.has(r.op)

// A part of a saved search the builder can't take apart (from the API or a plugin) stays as it is, so nothing is lost.
export const QUERY_PROP = ['__query', null, 'query']
// any custom field, named in the row itself; only fields given their own filter are listed by name
export const FIELD_PROP = ['__field', null, 'field']

/** A rule's property as the builder lists it: Curio's own by its catalog name, a field by its label. */
export const propLabel = (p) => p[1] ?? t(`filters.props.${p[0]}`)

const custom = (fields) => (fields || []).filter((f) => !BUILTIN_PROPS.some((p) => p[0] === f.key))
function fieldProp(f) {
  const t = f.type
  const kind = t === 'integer' || t === 'float' ? 'num' : t === 'date' || t === 'datetime' ? 'date' : t === 'boolean' ? 'bool' : t === 'single_select' || t === 'multi_select' ? 'select' : 'text'
  return [f.key, f.label, kind, f.options || []]
}

const indexed = (fields) => custom(fields).filter((f) => f.indexed !== false)

/** What the property list offers: the indexed fields with a filter of their own by name, any other through a Custom field row. */
export function propsWith(fields) {
  return [...BUILTIN_PROPS, ...indexed(fields).filter((f) => f.own_filter).map(fieldProp), FIELD_PROP]
}

/** The indexed fields a Custom field row offers, those without a filter of their own, in groups by where they come from: `[{ label, items }]`. */
export function fieldGroups(fields, nameOf = (o) => o) {
  const m = new Map()
  for (const f of indexed(fields).filter((f) => !f.own_filter)) {
    if (!m.has(f.owner)) m.set(f.owner, { label: nameOf(f.owner), items: [] })
    m.get(f.owner).items.push(f)
  }
  return [...m.values()]
}

/** Every property a query can name. */
export function allProps(fields) {
  return [...BUILTIN_PROPS, ...custom(fields).map(fieldProp)]
}

/** The name a rule puts in the query. */
export const ruleKey = (r) => (r.prop === FIELD_PROP[0] ? r.field || '' : r.prop)

export function ruleType(r, props) {
  if (r.prop === QUERY_PROP[0]) return 'query'
  return props.find((p) => p[0] === ruleKey(r))?.[2] || 'text'
}

/** The field holding one value for each of this one's, at the same place in its list, as a LoRA's strength for each LoRA. */
export const pairedField = (key, fields) => (fields || []).find((g) => g.pair === key && g.key !== key && g.indexed !== false) || null
const isNum = (v) => v !== '' && v != null && Number.isFinite(Number(v))
/** A range of the paired field for the value just named, as the query says it: ` WITH lora_strength BETWEEN 0.5 AND 1`, or '' without one. */
export function withRange(paired, min, max) {
  if (!paired || (!isNum(min) && !isNum(max))) return ''
  if (isNum(min) && isNum(max)) return ` WITH ${paired} BETWEEN ${Number(min)} AND ${Number(max)}`
  return isNum(min) ? ` WITH ${paired} >= ${Number(min)}` : ` WITH ${paired} <= ${Number(max)}`
}

/** A row holding rules of its own, matched as a whole. */
export function newGroup(props) {
  return { group: true, match: 'any', truth: 'true', rules: [newRule('name', props)] }
}

export function newRule(prop, props, field = '') {
  const r = { prop, field }
  const type = ruleType(r, props)
  const op = OPS[type][0][0]
  return { prop, field, op, value: LIST_OPS.has(op) ? [] : defaultValue(type), value2: '', unit: defaultUnit(type), pmin: '', pmax: '' }
}
/** A row moved between an operator taking a list of names and one taking words keeps what it can of its value. */
export function fitValue(r) {
  if (isListRule(r)) r.value = Array.isArray(r.value) ? r.value : String(r.value || '').trim() ? [String(r.value).trim()] : []
  else if (Array.isArray(r.value)) r.value = r.value[0] || ''
}
function defaultValue(type) {
  if (type === 'rating') return 5
  if (type === 'color') return '#38BDC9'
  if (type === 'shape') return 'landscape'
  if (type === 'kind') return 'image'
  if (type === 'bool') return 'true'
  if (type === 'yesno') return '1'
  return ''
}
function defaultUnit(type) {
  return type === 'size' ? 'MB' : type === 'duration' ? 'sec' : type === 'date' ? 'days' : ''
}

const quote = (v) => JSON.stringify(String(v))
/** One word or "quoted phrase" looked for everywhere: the full-text index, and folder names and extensions, which it doesn't hold. */
export function wordClause(raw) {
  const w = quote(String(raw).replace(/^"|"$/g, ''))
  return `(${w} OR folder CONTAINS ${w} OR ext CONTAINS ${w})`
}
/** Typed words, a "quoted phrase" counting as one. */
export const wordsOf = (text) => String(text || '').match(/"[^"]*"|\S+/g) || []
const isNumber = (v) => /^-?\d+(\.\d+)?$/.test(String(v).trim())

function ruleToClause(r, props, fields) {
  if (r.group) return rulesToQuery(r, fields)
  const type = ruleType(r, props)
  if (type === 'query') return String(r.value).trim() ? `(${r.value})` : ''
  if (type === 'words') {
    const groups = wordsOf(r.value).map(wordClause)
    if (!groups.length) return ''
    const q = groups.length > 1 ? `(${groups.join(' AND ')})` : groups[0]
    return r.op === 'NOT CONTAINS' ? `NOT ${q}` : q
  }
  const key = ruleKey(r)
  if (!key) return ''
  if (r.op.startsWith('IS ')) return `${key} ${r.op}`
  if (isListRule(r)) {
    const names = (Array.isArray(r.value) ? r.value : [r.value]).filter((v) => v !== '' && v != null)
    if (!names.length) return ''
    const list = `(${names.map(quote).join(', ')})`
    if (r.op === 'EXACTLY') return `(${key} HAS ALL ${list} AND tag_count = ${names.length})`
    return `${key} ${r.op} ${list}`
  }
  const empty = (v) => v === '' || v == null
  if (empty(r.value)) return ''
  const unit = UNIT_SUFFIX[r.unit] || ''
  const lit = (v) => {
    switch (type) {
      case 'num': return isNumber(v) ? String(v).trim() : null
      case 'size': case 'duration': return isNumber(v) ? `${String(v).trim()}${unit}` : null
      case 'rating': case 'bool': case 'yesno': case 'color': case 'shape': case 'kind': return String(v)
      case 'date': return String(v)
      default: return quote(v)
    }
  }
  if (r.op === 'WITHIN') return isNumber(r.value) ? `${key} WITHIN ${String(r.value).trim()}${unit || 'd'}` : ''
  const a = lit(r.value)
  if (a == null) return ''
  if (r.op === 'BETWEEN') {
    const b = empty(r.value2) ? null : lit(r.value2)
    return b == null ? '' : `${key} BETWEEN ${a} AND ${b}`
  }
  return `${key} ${r.op} ${a}` + (r.op === '=' ? withRange(pairedField(key, fields)?.key, r.pmin, r.pmax) : '')
}

export function rulesToQuery(s, fields) {
  const props = allProps(fields)
  const parts = s.rules.map((r) => ruleToClause(r, props, fields)).filter(Boolean)
  if (!parts.length) return ''
  const q = parts.join(s.match === 'any' ? ' OR ' : ' AND ')
  if (s.truth === 'false') return `NOT (${q})`
  return parts.length > 1 ? `(${q})` : q
}

export function groupsToQuery(groups, fields) {
  const parts = (groups || []).map((g) => rulesToQuery(g, fields)).filter(Boolean)
  return parts.join(' AND ')
}

// ------------------------------------------------------------------ reading a query back

/** Split on a word at the top level: outside quotes and brackets, and not BETWEEN's own AND. */
function splitTop(text, word) {
  const out = []
  let depth = 0
  let inQuote = false
  let start = 0
  let between = false
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if (inQuote) {
      if (c === '\\') i++
      else if (c === '"') inQuote = false
      continue
    }
    if (c === '"') inQuote = true
    else if (c === '(') depth++
    else if (c === ')') depth--
    else if (depth === 0 && /\s/.test(c)) {
      const rest = text.slice(i + 1)
      if (/^BETWEEN\s/i.test(rest)) between = true
      if (rest.slice(0, word.length).toUpperCase() === word && /\s/.test(rest[word.length] || '')) {
        if (word === 'AND' && between) {
          between = false
          continue
        }
        out.push(text.slice(start, i).trim())
        start = i + 1 + word.length
        i += word.length
      }
    }
  }
  out.push(text.slice(start).trim())
  return out
}

/** Whether a pair of brackets wraps the whole text. */
function wrapped(t) {
  if (!t.startsWith('(') || !t.endsWith(')')) return false
  let depth = 0
  let inQuote = false
  for (let i = 0; i < t.length; i++) {
    const c = t[i]
    if (inQuote) {
      if (c === '\\') i++
      else if (c === '"') inQuote = false
    } else if (c === '"') inQuote = true
    else if (c === '(') depth++
    else if (c === ')' && --depth === 0 && i < t.length - 1) return false
  }
  return true
}

// the last WITH, as a value before it may hold the word
const WITH_RANGE = /^(.+)\s+WITH\s+([\w.]+)\s+(?:BETWEEN\s+(-?[\d.]+)\s+AND\s+(-?[\d.]+)|>=\s*(-?[\d.]+)|<=\s*(-?[\d.]+))$/i
const CLAUSE = /^([\w.]+)\s+(IS NOT EMPTY|IS EMPTY|NOT CONTAINS|CONTAINS|STARTS WITH|ENDS WITH|MATCHES|WITHIN|BETWEEN|!=|<=|>=|=|<|>|~)\s*(.*)$/i
const ALIASES = { tags: 'tag', folders: 'folder', stars: 'rating', note: 'annotation', type: 'kind', source: 'url', colour: 'color' }

function unquote(v) {
  const t = v.trim()
  if (t.startsWith('"')) {
    try {
      return JSON.parse(t)
    } catch {
      return null
    }
  }
  return t
}

/** A number with its unit: "5mb" into 5 and MB. */
function splitUnit(v, units) {
  const m = v.trim().match(/^(-?[\d.]+)\s*([a-z]*)$/i)
  if (!m) return null
  const suffix = m[2].toLowerCase()
  if (!suffix) return { value: m[1], unit: null }
  const unit = units.find((u) => UNIT_SUFFIX[u] === suffix)
  return unit ? { value: m[1], unit } : null
}

function clauseToRule(text, props, listed) {
  const m = text.match(CLAUSE)
  if (!m) return null
  const key = ALIASES[m[1].toLowerCase()] || m[1]
  // a field without its own filter comes back as a "Custom field" row
  const generic = !listed.some((p) => p[0] === key) && props.some((p) => p[0] === key)
  // a name the rows don't know (path, hash) is kept as it is
  if (!props.some((p) => p[0] === key)) return null
  const prop = generic ? FIELD_PROP[0] : key
  const field = generic ? key : ''
  const op = m[2].toUpperCase()
  const raw = m[3]
  const type = ruleType({ prop, field }, props)
  if ((type === 'tags' || type === 'folders') && (op === '=' || op === '!=')) {
    const name = unquote(raw)
    return name == null || name === '' ? null : { prop, field, op: op === '=' ? 'IN' : 'HAS NONE', value: [name], value2: '', unit: '', pmin: '', pmax: '' }
  }
  if (!OPS[type].some((o) => o[0] === op)) return null
  const rule = { prop, field, op, value: '', value2: '', unit: defaultUnit(type), pmin: '', pmax: '' }
  if (op.startsWith('IS ')) return raw.trim() ? null : rule
  const units = type === 'size' ? SIZE_UNITS : type === 'duration' ? DURATION_UNITS : type === 'date' ? AGE_UNITS : null
  const one = (v) => {
    if (type === 'size' || type === 'duration') {
      const u = splitUnit(v, units)
      if (!u) return null
      if (u.unit) rule.unit = u.unit
      // a bare size is bytes and a bare duration seconds; only seconds have a row for that
      if (!u.unit && type === 'size') return null
      return u.value
    }
    if (type === 'rating') return /^\d$/.test(v.trim()) ? Number(v) : null
    return unquote(v)
  }
  if (op === 'WITHIN') {
    const u = splitUnit(raw, AGE_UNITS)
    if (!u || !u.unit) return null
    rule.value = u.value
    rule.unit = u.unit
    return rule
  }
  if (op === 'BETWEEN') {
    const pair = raw.split(/\s+AND\s+/i)
    if (pair.length !== 2) return null
    rule.value = one(pair[0])
    rule.value2 = one(pair[1])
    return rule.value == null || rule.value2 == null ? null : rule
  }
  rule.value = one(raw)
  return rule.value == null ? null : rule
}

const NEGATED = { '=': '!=', '!=': '=', CONTAINS: 'NOT CONTAINS', 'NOT CONTAINS': 'CONTAINS', 'IS EMPTY': 'IS NOT EMPTY', 'IS NOT EMPTY': 'IS EMPTY', IN: 'HAS NONE', 'HAS NONE': 'IN' }
// a list's operator, and how each of its items reads as a row
const LISTS = { IN: ['=', 'any'], 'NOT IN': ['!=', 'all'], 'HAS ANY': ['=', 'any'], 'HAS ALL': ['=', 'all'], 'HAS NONE': ['!=', 'all'] }
const STRING = String.raw`"(?:[^"\\]|\\.)*"`
const WORDS = new RegExp(String.raw`^(${STRING})\s+OR\s+folder\s+CONTAINS\s+\1\s+OR\s+ext\s+CONTAINS\s+\1$`, 'i')
const LONE = new RegExp(`^(${STRING})$`)

/** A list's items, split on commas outside quotes. */
function listItems(text) {
  const out = []
  let inQuote = false
  let start = 0
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if (inQuote) {
      if (c === '\\') i++
      else if (c === '"') inQuote = false
    } else if (c === '"') inQuote = true
    else if (c === ',') {
      out.push(text.slice(start, i).trim())
      start = i + 1
    }
  }
  out.push(text.slice(start).trim())
  return out.filter(Boolean)
}

/** Words looked for everywhere, as the search box writes them, or a lone quoted word. */
function wordsRule(t) {
  const m = t.match(WORDS) || t.match(LONE)
  if (!m) return null
  let w
  try {
    w = JSON.parse(m[1])
  } catch {
    return null
  }
  return { prop: TEXT_PROP[0], field: '', op: 'CONTAINS', value: /\s/.test(w) ? `"${w}"` : w, value2: '', unit: '' }
}

// how a list of tags or folders reads as one row
const NAME_LISTS = { IN: 'IN', 'HAS ANY': 'IN', 'HAS ALL': 'HAS ALL', 'NOT IN': 'HAS NONE', 'HAS NONE': 'HAS NONE' }
function nameList(name, how, items, ctx) {
  const prop = ALIASES[name.toLowerCase()] || name
  const type = ruleType({ prop, field: '' }, ctx.props)
  if (type !== 'tags' && type !== 'folders') return null
  const names = listItems(items).map(unquote)
  if (!names.length || names.some((n) => n == null || n === '')) return null
  return { prop, field: '', op: NAME_LISTS[how.toUpperCase().replace(/\s+/g, ' ')], value: names, value2: '', unit: '', pmin: '', pmax: '' }
}
// tags that are exactly these: all of them, and no more
const EXACT = /^([\w.]+)\s+HAS\s+ALL\s*\((.*)\)\s+AND\s+tag_count\s*=\s*(\d+)$/is
function exactTags(t, ctx) {
  const m = t.match(EXACT)
  const r = m && nameList(m[1], 'HAS ALL', m[2], ctx)
  return r?.prop === 'tag' && r.value.length === Number(m[3]) ? { ...r, op: 'EXACTLY' } : null
}

const queryRow = (q) => ({ prop: QUERY_PROP[0], field: '', op: '=', value: q, value2: '', unit: '' })
const asRow = (g) => (g.rules.length === 1 && g.truth === 'true' ? g.rules[0] : { group: true, ...g })

/** A group from its text: what joins its parts at the top, and each part as a row, a row group, or kept as it is. */
function groupOf(text, ctx) {
  let t = text.trim()
  let truth = 'true'
  const neg = t.match(/^NOT\s*(\(.*\))$/is)
  if (neg && wrapped(neg[1])) {
    truth = 'false'
    t = neg[1]
  }
  while (wrapped(t)) t = t.slice(1, -1).trim()
  const words = wordsRule(t) || exactTags(t, ctx)
  if (words) return { match: 'all', truth, rules: [words] }
  const ors = splitTop(t, 'OR')
  const match = ors.length > 1 ? 'any' : 'all'
  const rules = []
  for (const part of ors.length > 1 ? ors : splitTop(t, 'AND')) {
    const r = partToRule(part, ctx) || queryRow(part)
    // a group joined the way its parent is takes no row of its own
    if (r.group && r.truth === 'true' && r.match === match) rules.push(...r.rules)
    else rules.push(r)
  }
  const [lone] = rules
  if (rules.length === 1 && lone.group && lone.truth === 'true') return { match: lone.match, truth, rules: lone.rules }
  return { match, truth, rules }
}

function partToRule(text, ctx) {
  const t = text.trim()
  if (splitTop(t, 'OR').length > 1 || splitTop(t, 'AND').length > 1) return asRow(groupOf(t, ctx))
  const r = clauseToRule(t, ctx.props, ctx.listed)
  if (r) return r
  const range = t.match(WITH_RANGE)
  if (range) {
    const p = clauseToRule(range[1], ctx.props, ctx.listed)
    if (p?.op === '=' && pairedField(ruleKey(p), ctx.fields)?.key === range[2]) return Object.assign(p, { pmin: range[3] ?? range[5] ?? '', pmax: range[4] ?? range[6] ?? '' })
  }
  const words = wordsRule(t)
  if (words) return words
  const list = t.match(/^([\w.]+)\s+(NOT\s+IN|IN|HAS\s+ANY|HAS\s+ALL|HAS\s+NONE)\s*\((.*)\)$/is)
  if (list) {
    const named = nameList(list[1], list[2], list[3], ctx)
    if (named) return named
    const [op, match] = LISTS[list[2].toUpperCase().replace(/\s+/g, ' ')]
    const rows = listItems(list[3]).map((v) => clauseToRule(`${list[1]} ${op} ${v}`, ctx.props, ctx.listed))
    return rows.length && rows.every(Boolean) ? asRow({ match, truth: 'true', rules: rows }) : null
  }
  const not = t.match(/^NOT\s+(.*)$/is)
  if (not && !wrapped(not[1].trim())) {
    const inner = partToRule(not[1], ctx)
    if (!inner) return null
    if (inner.group) return { ...inner, truth: inner.truth === 'true' ? 'false' : 'true' }
    const flipped = NEGATED[inner.op]
    if (flipped && OPS[ruleType(inner, ctx.props)].some((o) => o[0] === flipped)) return { ...inner, op: flipped }
    return { group: true, match: 'all', truth: 'false', rules: [inner] }
  }
  if (wrapped(t) || /^NOT\s*\(/i.test(t)) return asRow(groupOf(t, ctx))
  return null
}

/** The builder's rows for a saved query: every part it can show as rules, any other kept as it is. */
export function queryToRules(query, fields) {
  const props = allProps(fields)
  const t = (query || '').trim()
  if (!t) return { match: 'all', truth: 'true', rules: [] }
  return groupOf(t, { props, listed: propsWith(fields), fields })
}

/** The builder's flat AND-joined groups for a saved query; any nested sub-group falls back to a raw query row. */
export function queryToGroups(query, fields) {
  const t = (query || '').trim()
  if (!t) return []
  const root = queryToRules(t, fields)
  const flat = (rules) => rules.map((r) => (r.group ? queryRow(rulesToQuery(r, fields)) : r))
  if (root.truth === 'true' && root.match === 'all' && root.rules.some((r) => r.group)) {
    const out = []
    let plain = []
    const flush = () => {
      if (plain.length) out.push({ match: 'all', truth: 'true', rules: plain })
      plain = []
    }
    for (const r of root.rules) {
      if (r.group) {
        flush()
        out.push({ match: r.match, truth: r.truth, rules: flat(r.rules) })
      } else plain.push(r)
    }
    flush()
    return out
  }
  return [{ match: root.match, truth: root.truth, rules: flat(root.rules) }]
}
