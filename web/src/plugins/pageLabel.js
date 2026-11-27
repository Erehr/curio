// The host label of a plugin's own origin, worked out as `plugins::page_label` in src/core/plugins/mod.rs does: the id fit for a host name, and an FNV-1a hash of the id as given.
export function pageLabel(id) {
  let h = 0x811c9dc5
  for (const b of new TextEncoder().encode(id)) h = Math.imul(h ^ b, 0x01000193) >>> 0
  const hash = h.toString(16).padStart(8, '0')
  const readable = id.toLowerCase().replace(/[^a-z0-9]/g, '-').slice(0, 40).replace(/^-+|-+$/g, '')
  return readable ? `${readable}-${hash}` : `p${hash}`
}

/** Where a plugin's pages are served: an origin of its own. */
export function pageOrigin(id, windows) {
  return windows ? `http://plugin.${pageLabel(id)}.localhost` : `plugin://${pageLabel(id)}`
}
