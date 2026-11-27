// The Random view keeps one order until asked to reshuffle: leaving and coming
// back, or a change to an item, doesn't deal the cards again. Items no longer in the results
// drop out; new ones join at the end.
export function arrangeLike(order, ids, aspects) {
  const aspectOf = new Map()
  for (let i = 0; i < ids.length; i++) aspectOf.set(ids[i], aspects[i])
  const kept = order.filter((id) => aspectOf.has(id))
  const known = new Set(order)
  const added = Array.from(ids).filter((id) => !known.has(id))
  const all = [...kept, ...added]
  return {
    ids: Uint32Array.from(all),
    aspects: Float32Array.from(all, (id) => aspectOf.get(id)),
    order: [...order, ...added],
  }
}
