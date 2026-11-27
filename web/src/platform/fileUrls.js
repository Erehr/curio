// The addresses the window loads a library's files from, through Curio's thumb protocol: thumbnails, originals, sized previews, media for the player, and text.
// Windows' web view reaches a custom protocol as http://<name>.localhost
const isWin = navigator.userAgent.includes('Windows')
export const BASE = isWin ? 'http://thumb.localhost' : 'thumb://localhost'

export const thumbUrl = (uid, ver) => `${BASE}/t/${uid}/${ver}`
export const originalUrl = (id, ver) => `${BASE}/f/${id}${ver ? '?v=' + ver : ''}`
export const previewUrl = (id, size, ver) => `${BASE}/p/${id}/${size}${ver ? '?v=' + ver : ''}`
// Linux's web view can't play media from the custom scheme, so the program offers a local address
let mediaBase = null
export const setMediaBase = (base) => (mediaBase = base || null)
export const playUrl = (id) => `${mediaBase || BASE}/v/${id}`
export const textUrl = (id) => `${BASE}/x/${id}`
