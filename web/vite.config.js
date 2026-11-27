import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { locales } from './locales.vite.js'

// CURIO_PSEUDO=1 (build.rs sets it for debug builds) adds the pseudo-language, into a folder of its own so a release never embeds it
const PSEUDO = process.env.CURIO_PSEUDO === '1'
export const outDir = PSEUDO ? '../target/web-dist-debug' : '../target/web-dist'
const require = createRequire(import.meta.url)

const assets = {
  name: 'curio-static-assets',
  generateBundle() {
    if (PSEUDO) this.emitFile({ type: 'asset', fileName: 'pseudo.txt', source: 'This window was built for debugging, with the pseudo-language; a release never is.\n' })
    this.emitFile({ type: 'asset', fileName: 'tabler.json', source: readFileSync(require.resolve('@iconify-json/tabler/icons.json')) })
    // opt-in base stylesheet and overlay scrollbars served to plugin pages under /curio/ui/
    const css = readFileSync(new URL('src/shared/curio.css', import.meta.url), 'utf8') + '\n' + readFileSync(new URL('src/shared/pluginPage.css', import.meta.url), 'utf8')
    this.emitFile({ type: 'asset', fileName: 'plugin/curio.css', source: css })
    this.emitFile({ type: 'asset', fileName: 'plugin/overlay-scroll.js', source: readFileSync(new URL('src/shared/overlayScroll.js', import.meta.url)) })
  },
}

export default defineConfig({
  plugins: [vue(), assets, ...locales({ files: ['app', 'openvue'], pseudo: PSEUDO, runtimeOnly: true })],
  base: './',
  define: { 'import.meta.env.CURIO_PSEUDO': JSON.stringify(PSEUDO) },
  build: {
    outDir,
    emptyOutDir: true,
    target: 'es2021',
    chunkSizeWarningLimit: 6000,
    minify: process.env.NODE_ENV === 'development' ? false : 'esbuild',
  },
})
