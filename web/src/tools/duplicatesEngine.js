// The duplicate finder's page logic, without the page: groups as the search answers them, which member each rule keeps, how members match the kept one, and what removing the others frees.

export const WAYS = ['content', 'looks', 'video', 'sound', 'archive']
/** The loosest similarity the search takes for each way, and the one unless set. */
export const RANGE = { looks: [0.8, 0.9], video: [0.7, 0.9], sound: [0.4, 0.7], archive: [0.5, 0.9] }

/** The settings as last used, with what is missing filled in. */
export function complete(v = {}) {
  const ways = v.ways || {}
  const way = (w, on) => ({ on: ways[w]?.on ?? on, t: Math.min(1, Math.max(RANGE[w]?.[0] ?? 1, ways[w]?.t ?? RANGE[w]?.[1] ?? 1)) })
  return {
    ways: { content: way('content', true), looks: { ...way('looks', true), others: ways.looks?.others ?? true }, video: way('video', false), sound: way('sound', false), archive: way('archive', false) },
    scope: ['library', 'shown', 'selected', 'folder'].includes(v.scope) ? v.scope : 'library',
    against: v.against === 'library' ? 'library' : 'each',
    minKb: Math.max(0, Number(v.minKb) || 0),
    keep: KEEP_RULES.some((r) => r.value === v.keep) ? v.keep : 'biggest',
    combine: v.combine !== false,
    details: v.details === true,
  }
}

/** What `duplicates.find` takes for these settings, the scope aside. */
export function findArgs(s) {
  const on = WAYS.filter((w) => s.ways[w].on)
  const thresholds = Object.fromEntries(on.filter((w) => w !== 'content').map((w) => [w, s.ways[w].t]))
  return { ways: on, thresholds, ...(s.ways.looks.on && !s.ways.looks.others ? { lookKinds: ['image'] } : {}), ...(s.minKb ? { minSize: Math.round(s.minKb * 1024) } : {}) }
}

/** A search's group made ready for the page: its members with their items, the links between them by pair, and whether all of them are identical. */
export function prepare(g, items, key) {
  const members = g.members.map((m) => ({ ...m, item: items.get(m.id) || null }))
  const links = new Map()
  for (const [a, b, way, score] of g.links) links.set(pairKey(a, b), { way, score })
  const identical = members.every((m) => m.same != null && m.same === members[0].same)
  return { key, members, links, identical, keep: 0, remove: new Set(), include: identical, touched: false }
}

const pairKey = (a, b) => (a < b ? `${a}-${b}` : `${b}-${a}`)

/** How member `i` matches member `j`: identical, or the link between them. */
export function relation(g, i, j) {
  if (i === j) return null
  const a = g.members[i]
  const b = g.members[j]
  if (a.same != null && a.same === b.same) return { way: 'content', score: 1 }
  return g.links.get(pairKey(i, j)) || null
}

/** The best similarity in a group that isn't identical, for sorting and its label. */
export function likeness(g) {
  if (g.identical) return 1
  let best = 0
  for (const l of g.links.values()) best = Math.max(best, l.score)
  return best || 1
}

const pixels = (it) => (it?.width || 0) * (it?.height || 0)
const byBiggest = (a, b) => pixels(b.item) - pixels(a.item) || (b.item?.size || 0) - (a.item?.size || 0) || (a.item?.imported || 0) - (b.item?.imported || 0)

/** How each rule picks the item to keep: the first after sorting, ties going to the biggest picture and then the oldest. */
export const KEEP_RULES = [
  { value: 'biggest', order: byBiggest },
  { value: 'largest', order: (a, b) => (b.item?.size || 0) - (a.item?.size || 0) || byBiggest(a, b) },
  { value: 'smallest', order: (a, b) => (a.item?.size || 0) - (b.item?.size || 0) || byBiggest(a, b) },
  { value: 'oldest', order: (a, b) => (a.item?.imported || 0) - (b.item?.imported || 0) || byBiggest(a, b) },
  { value: 'newest', order: (a, b) => (b.item?.imported || 0) - (a.item?.imported || 0) || byBiggest(a, b) },
  { value: 'rated', order: (a, b) => (b.item?.rating || 0) - (a.item?.rating || 0) || (b.item?.tags?.length || 0) - (a.item?.tags?.length || 0) || byBiggest(a, b) },
  { value: 'store', order: (a, b) => Number(b.store) - Number(a.store) || byBiggest(a, b) },
  { value: 'linked', order: (a, b) => Number(a.store) - Number(b.store) || byBiggest(a, b) },
  { value: 'folder', order: byBiggest },
]

/** The member a rule keeps; with `folder`, one in that folder first. */
export function keeper(g, rule, folder = null) {
  const r = KEEP_RULES.find((x) => x.value === rule) || KEEP_RULES[0]
  const order = rule === 'folder' && folder != null ? (a, b) => Number(inFolder(b, folder)) - Number(inFolder(a, folder)) || byBiggest(a, b) : r.order
  const ranked = g.members.map((m, i) => ({ ...m, i })).filter((m) => m.item).sort(order)
  return ranked.length ? ranked[0].i : 0
}

const inFolder = (m, folder) => (m.item?.folders || []).some((f) => f.id === folder)

/** The members moved to the trash: all but the kept one and those kept too. */
export const removed = (g) => g.members.map((_, i) => i).filter((i) => i !== g.keep && !g.remove.has(i))

/** What removing a group's copies frees once the trash is emptied: files in the library's own folder only, each file once, none that a kept member is a hard link of. `stay` counts copies linked in place, whose files stay where they are. */
export function freed(g) {
  const gone = new Set(removed(g))
  const keptFiles = new Set(g.members.filter((m, i) => !gone.has(i) && m.file != null).map((m) => m.file))
  const counted = new Set()
  let bytes = 0
  let stay = 0
  for (const i of gone) {
    const m = g.members[i]
    if (!m.store) {
      stay++
      continue
    }
    if (m.file != null) {
      if (keptFiles.has(m.file) || counted.has(m.file)) continue
      counted.add(m.file)
    }
    bytes += m.item?.size || 0
  }
  return { bytes, stay, count: gone.size }
}

/** What `duplicates.resolve` takes for an included group: the kept item, the copies removed, and those of them found identical to it. */
export function settleArgs(g) {
  const gone = removed(g)
  return { keep: g.members[g.keep].id, remove: gone.map((i) => g.members[i].id), identical: gone.filter((i) => relation(g, i, g.keep)?.way === 'content').map((i) => g.members[i].id) }
}

export const SORTS = [
  { value: 'space', order: (a, b) => freed(b).bytes - freed(a).bytes || b.members.length - a.members.length },
  { value: 'copies', order: (a, b) => b.members.length - a.members.length || freed(b).bytes - freed(a).bytes },
  { value: 'alike', order: (a, b) => likeness(a) - likeness(b) },
  { value: 'name', order: (a, b) => (a.members[a.keep].item?.name || '').localeCompare(b.members[b.keep].item?.name || '') },
]


/** How a member matches the kept one: `kept`, `alike` (no link between them), `identical`, `samePages`, else `way` with the link's way and score. */
export function matchOf(g, i) {
  if (i === g.keep) return { is: 'kept' }
  const r = relation(g, i, g.keep)
  if (!r) return { is: 'alike' }
  if (r.way === 'content') return { is: 'identical' }
  if (r.way === 'archive' && r.score >= 1) return { is: 'samePages' }
  return { is: 'way', way: r.way, score: r.score }
}

export function dateText(ms) {
  if (!ms) return ''
  const d = new Date(ms)
  return `${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`
}
