// The window hears `plugin.run` with the items it was opened for; everything else goes through the API.
const curio = Curio.connect()
const $ = (id) => document.getElementById(id)
const size = (b) => (b > 1048576 ? `${(b / 1048576).toFixed(1)} MB` : `${Math.round(b / 1024)} KB`)

let ids = []
async function show(list) {
  ids = list
  const items = ids.length ? await curio.item.getMany({ ids }) : []
  $('table').hidden = !items.length
  $('rows').replaceChildren(...items.map((i) => {
    const tr = document.createElement('tr')
    for (const [text, cls] of [[i.name, ''], [i.kind, ''], [i.width ? `${i.width} × ${i.height}` : '', 'mono'], [size(i.size || 0), 'mono']]) {
      const td = document.createElement('td')
      if (cls) td.className = cls
      td.textContent = text
      tr.append(td)
    }
    return tr
  }))
  $('hint').hidden = items.length > 0
  $('count').textContent = items.length ? `${items.length} item${items.length > 1 ? 's' : ''}` : ''
  $('size').textContent = items.length ? `${size(items.reduce((n, i) => n + (i.size || 0), 0))} in all` : ''
}
curio.on('plugin.run', (e) => show(e.data.items || []))
// items dragged from the grid, or files from the system, which Curio adds first
curio.on('plugin.drop', (e) => show([...new Set([...ids, ...e.data.items])]))
$('notify').addEventListener('click', () => curio.ui.notify({ text: 'Hello from the example window' }))
$('select').addEventListener('click', () => curio.selection.set({ ids }))
