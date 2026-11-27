// Markdown drawn in the window itself, kept to an allowlist so a file can't run script, restyle the window or fetch anything.
import { marked } from 'marked'

export const MARKDOWN = ['md', 'markdown']
export const MAX_RENDER = 400000

const KEEP = new Set(['a', 'abbr', 'b', 'blockquote', 'br', 'code', 'dd', 'del', 'details', 'div', 'dl', 'dt', 'em', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'img', 'input', 'ins', 'kbd', 'li', 'mark', 'ol', 'p', 'pre', 's', 'samp', 'small', 'span', 'strong', 'sub', 'summary', 'sup', 'table', 'tbody', 'td', 'tfoot', 'th', 'thead', 'tr', 'u', 'ul', 'var'])
// gone with what they hold; any other tag gives way to its content
const DROP = new Set(['script', 'style', 'iframe', 'frame', 'frameset', 'object', 'embed', 'applet', 'form', 'button', 'select', 'textarea', 'option', 'svg', 'math', 'template', 'noscript', 'link', 'meta', 'base', 'title', 'head', 'video', 'audio', 'source', 'track', 'picture', 'canvas', 'map', 'area', 'dialog', 'portal'])
const ATTRS = { a: ['href', 'title'], img: ['src', 'alt', 'title', 'width', 'height'], td: ['align', 'colspan', 'rowspan'], th: ['align', 'colspan', 'rowspan'], ol: ['start'], input: ['type', 'checked', 'disabled'], details: ['open'] }
const EVERYWHERE = ['title', 'align']
const WEB = /^https?:\/\//i
// pictures held in the file itself; anything else would be read from somewhere
const INLINE_IMAGE = /^data:image\/(png|jpe?g|gif|webp|avif|bmp);base64,/i

/** A heading's anchor as `[text](#anchor)` links write it. */
export const slug = (text) => text.trim().toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s+/g, '-')

function clean(node, doc) {
  for (const el of [...node.children]) {
    const tag = el.tagName.toLowerCase()
    if (DROP.has(tag) || (tag === 'input' && el.getAttribute('type') !== 'checkbox')) {
      el.remove()
      continue
    }
    if (!KEEP.has(tag)) {
      clean(el, doc)
      el.replaceWith(...el.childNodes)
      continue
    }
    const allowed = ATTRS[tag] || []
    for (const a of [...el.attributes]) if (!allowed.includes(a.name) && !EVERYWHERE.includes(a.name) && !(tag === 'code' && a.name === 'class' && /^language-[\w-]+$/.test(a.value))) el.removeAttribute(a.name)
    if (tag === 'a') {
      const href = el.getAttribute('href') || ''
      if (!WEB.test(href) && !href.startsWith('#')) {
        el.removeAttribute('href')
        if (href && !el.title) el.title = href
      }
    } else if (tag === 'img' && !INLINE_IMAGE.test(el.getAttribute('src') || '')) {
      const alt = doc.createElement('span')
      alt.className = 'md-image'
      alt.textContent = el.getAttribute('alt') || el.getAttribute('src') || ''
      el.replaceWith(alt)
      continue
    } else if (tag === 'input') {
      el.setAttribute('disabled', '')
    } else if (/^h[1-6]$/.test(tag)) {
      el.dataset.anchor = slug(el.textContent)
    }
    clean(el, doc)
  }
}

/** The file's Markdown as safe HTML: web links only (opened outside), no pictures but those inside the file. */
export function renderMarkdown(text) {
  const html = marked.parse(text.slice(0, MAX_RENDER), { breaks: true, async: false })
  // a parsed document is inert: nothing in it loads or runs
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html')
  clean(doc.body, doc)
  return doc.body.innerHTML
}
