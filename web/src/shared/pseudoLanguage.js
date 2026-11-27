// The pseudo-language: English with accented letters, a third longer and in brackets, so text that skipped the catalog stays plain and text that clips shows it.
// Debug builds list it; a release never does.
export const PSEUDO = 'en-XA'

const PLAIN = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
const ACCENTED = 'àƀçđéƒĝĥîĵķĺɱñöþǫŕšţûṽŵẋýžÀƁÇĐÉƑĜĤÎĴĶĹṀÑÖÞǪŔŠŢÛṼŴẊÝŽ'
const accent = (c) => {
  const i = PLAIN.indexOf(c)
  return i < 0 ? c : [...ACCENTED][i]
}

// where a placeholder starting at `i` ends: `{name}`, `{'{'}`
function placeholderEnd(s, i) {
  let quoted = false
  for (let j = i + 1; j < s.length; j++) {
    if (s[j] === "'") quoted = !quoted
    else if (s[j] === '}' && !quoted) return j + 1
  }
  return s.length
}

// where a linked message starting at `i` ends: `@:key`, `@.lower:key`, `@:{'key'}`, `@:(key)`
function linkEnd(s, i) {
  let j = i + 1
  while (s[j] === '.') {
    j++
    while (/\w/.test(s[j] || '')) j++
  }
  if (s[j] !== ':') return i + 1
  j++
  if (s[j] === '{') return placeholderEnd(s, j)
  if (s[j] === '(') return s.indexOf(')', j) + 1 || s.length
  while (/[\w.-]/.test(s[j] || '')) j++
  return j
}

function pseudoForm(form) {
  let out = ''
  let letters = 0
  for (let i = 0; i < form.length; ) {
    const c = form[i]
    const end = c === '\\' ? i + 2 : c === '{' ? placeholderEnd(form, i) : c === '@' ? linkEnd(form, i) : i + 1
    if (end === i + 1 && c !== '\\') {
      out += accent(c)
      if (/[a-z]/i.test(c)) letters++
    } else out += form.slice(i, end)
    i = end
  }
  const lead = out.match(/^\s*/)[0]
  const trail = out.match(/\s*$/)[0]
  return `${lead}[${out.trim()}${letters ? ' ' + '·'.repeat(Math.ceil(letters / 3)) : ''}]${trail}`
}

/** One message in the pseudo-language: only literal text changes; placeholders, linked messages, escapes and the `|` between plural forms stay. */
export function pseudoMessage(s) {
  const forms = []
  let start = 0
  for (let i = 0; i < s.length; i++) {
    if (s[i] === '\\') i++
    else if (s[i] === '{') i = placeholderEnd(s, i) - 1
    else if (s[i] === '|') {
      forms.push(s.slice(start, i))
      start = i + 1
    }
  }
  forms.push(s.slice(start))
  return forms.map(pseudoForm).join('|')
}

/** A whole catalog in the pseudo-language. */
export function pseudoCatalog(v) {
  if (typeof v === 'string') return pseudoMessage(v)
  if (Array.isArray(v)) return v.map(pseudoCatalog)
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, pseudoCatalog(x)]))
  return v
}
