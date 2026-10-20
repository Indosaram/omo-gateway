import { defineConfig } from '/Users/indo/code/project/omon-gateway/web/node_modules/vite/dist/node/index.js'
import react from '/Users/indo/code/project/omon-gateway/web/node_modules/@vitejs/plugin-react/dist/index.js'

export default defineConfig({
  root: '/Users/indo/code/project/omon-gateway/web',
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: 5199,
    strictPort: true,
    fs: { allow: ['/Users/indo/code/project/omon-gateway/web'] },
    proxy: {
      '/api': { target: 'http://127.0.0.1:9121', ws: true },
    },
  },
})
