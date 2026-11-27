// A tab queries the library and draws items using curio.js and opt-in curio.css.
const curio = Curio.connect()
const $ = (id) => document.getElementById(id)
const urls = []
let seq = 0

function clearUrls() {
  while (urls.length) URL.revokeObjectURL(urls.pop())
}

async function load() {
  const n = ++seq
  const min = Number($('min').value)
  const { items } = await curio.item.query({ query: `kind = image AND rating >= ${min}`, sort: 'rating', limit: 200 })
  if (n !== seq) return
  clearUrls()
  $('count').textContent = `${items.length} picture${items.length === 1 ? '' : 's'}`
  $('grid').replaceChildren(...items.map((item) => {
    const btn = document.createElement('button')
    btn.className = 'tile'
    btn.title = item.name
    const img = document.createElement('img')
    img.alt = item.name
    btn.append(img)
    // clicking selects it in Curio so the Inspector shows it; double-clicking opens it in the viewer
    btn.onclick = () => {
      $('grid').querySelectorAll('.sel').forEach((el) => el.classList.remove('sel'))
      btn.classList.add('sel')
      curio.selection.set({ ids: [item.id] })
    }
    btn.ondblclick = () => curio.viewer.open({ id: item.id })
    curio.item.thumbnail({ id: item.id, size: 320 }).then((blob) => {
      if (n !== seq || !blob) return
      const u = URL.createObjectURL(blob)
      urls.push(u)
      img.src = u
    }).catch(() => {})
    return btn
  }))
}

$('min').addEventListener('change', load)
curio.on('items.changed', load)
curio.on('plugin.beforeExit', clearUrls)
load()
