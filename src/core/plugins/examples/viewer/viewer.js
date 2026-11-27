// A viewer page hears `plugin.item` with the item to show, reads its file by address, and puts its own buttons in Curio's toolbar.
const curio = Curio.connect()
const box = document.getElementById('lines')
const MAX_LINES = 5000
let lines = []
let onlyProblems = false

function draw() {
  const shown = lines.map((t, i) => [i + 1, t]).filter(([, t]) => !onlyProblems || /warn|error/i.test(t))
  box.className = ''
  box.replaceChildren(...shown.map(([n, t]) => {
    const row = document.createElement('div')
    row.className = 'l' + (/error/i.test(t) ? ' error' : /warn/i.test(t) ? ' warn' : '')
    const num = document.createElement('span')
    num.className = 'n'
    num.textContent = n
    row.append(num, t)
    return row
  }))
  const problems = lines.filter((t) => /warn|error/i.test(t)).length
  curio.viewer.setTools({ labels: [{ id: 'lines', text: `${lines.length} lines, ${problems} warnings or errors` }], buttons: [{ id: 'problems', label: 'Only warnings and errors', icon: 'alert-triangle', on: onlyProblems }] })
}

curio.on('plugin.item', async (e) => {
  const { url } = await curio.item.fileUrl({ id: e.data.id })
  const text = await fetch(url).then((r) => r.text())
  lines = text.split(/\r?\n/).slice(0, MAX_LINES)
  draw()
})
curio.on('plugin.tool', () => {
  onlyProblems = !onlyProblems
  draw()
})
