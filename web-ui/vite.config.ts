import { URL, fileURLToPath } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import wasm from "vite-plugin-wasm"
// import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueJsx(),
    wasm(),
    // vueDevTools(),
  ],
  assetsInclude: ['**/*.wasm'],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
  build: {
    target: 'esnext',
  },
  // dev proxy
  server: {
    proxy: {
      '/ws': {
        target: 'http://localhost:9000',
        ws: true,
        changeOrigin: true,
      },
      '/ensembles': {
        target: 'http://localhost:9001',
        changeOrigin: true,
      },
    },
  },
})
