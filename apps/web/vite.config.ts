import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

const apiProxy = {
  '/api': {
    target: 'http://127.0.0.1:3000',
    changeOrigin: true,
  },
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: apiProxy,
  },
  preview: {
    host: 'localhost',
    port: 5173,
    strictPort: true,
    proxy: apiProxy,
  },
})
