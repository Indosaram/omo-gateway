import { defineConfig } from '/Users/indo/code/project/omon-gateway/web/node_modules/vite/dist/node/index.js'
import react from '/Users/indo/code/project/omon-gateway/web/node_modules/@vitejs/plugin-react/dist/index.js'

// Unique ports for the ui-final pass so it can run beside the earlier ui/ harness (5199/9121).
export default defineConfig({
  root: '/Users/indo/code/project/omon-gateway/web',
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: 5299,
    strictPort: true,
    fs: { allow: ['/Users/indo/code/project/omon-gateway/web'] },
    proxy: {
      '/api': { target: 'http://127.0.0.1:9131', ws: true },
    },
  },
})
