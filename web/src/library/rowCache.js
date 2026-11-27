// The rows of items on show, kept in a Map whose keys run from the one used longest ago to the one used last.

/** Marks a row as just used, so it is dropped last. */
export function touch(map, key) {
  const row = map.get(key)
  if (row === undefined) return
  map.delete(key)
  map.set(key, row)
}

/** Drops the rows used longest ago until `keep` are left, sparing those in `pinned`. */
export function trimOldest(map, keep, pinned) {
  let over = map.size - keep
  for (const key of map.keys()) {
    if (over <= 0) break
    if (pinned.has(key)) continue
    map.delete(key)
    over--
  }
}
