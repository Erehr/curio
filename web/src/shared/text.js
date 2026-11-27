// Search-phrase matching shared by the sidebar filter, popups and menus.
// A phrase matches where it appears whole; otherwise each of its words must begin a word of the name, in order, so "night sc" finds "Night scenes" but "tag" never finds "Containing" or "the… again" letter by letter.

const isWordStart = (name, i) => i === 0 || /[\s\-_./()[\]›,:]/.test(name[i - 1]) || (/[a-z]/.test(name[i - 1]) && /[A-Z]/.test(name[i]))

/** Indexes of the matched letters, or null. */
function find(name, q) {
  const n = name.toLowerCase()
  const t = q.toLowerCase().trim()
  const at = n.indexOf(t)
  if (at >= 0) return Array.from({ length: t.length }, (_, i) => at + i)
  const words = t.split(/\s+/).filter(Boolean)
  if (words.length < 2) return null
  const hits = []
  let from = 0
  for (const w of words) {
    let i = n.indexOf(w, from)
    while (i >= 0 && !isWordStart(name, i)) i = n.indexOf(w, i + 1)
    if (i < 0) return null
    for (let j = 0; j < w.length; j++) hits.push(i + j)
    from = i + w.length
  }
  return hits
}

export const fuzzy = (name, q) => !q || find(name, q) != null

/** Whole-phrase matches rank above spread-out ones, and earlier ones above later ones. */
export function fuzzyRank(name, q) {
  const at = name.toLowerCase().indexOf(q.toLowerCase())
  return at >= 0 ? at : 1000
}

/** The name split into plain and matching pieces. */
export function matchParts(name, q) {
  if (!q) return [{ t: name, hit: false }]
  const hits = find(name, q.trim())
  if (!hits || !hits.length) return [{ t: name, hit: false }]
  const set = new Set(hits)
  const out = []
  for (let i = 0; i < name.length; i++) {
    const hit = set.has(i)
    const last = out[out.length - 1]
    if (last && last.hit === hit) last.t += name[i]
    else out.push({ t: name[i], hit })
  }
  return out
}
