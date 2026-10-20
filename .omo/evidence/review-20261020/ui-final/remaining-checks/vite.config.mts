import { defineConfig } from '/Users/indo/code/project/omon-gateway/web/node_modules/vite/dist/node/index.js'
import react from '/Users/indo/code/project/omon-gateway/web/node_modules/@vitejs/plugin-react/dist/index.js'

// Ports 5399/9231 are unique to the remaining-checks pass so it cannot collide with ui-final
// (5299/9131) or the earlier ui/ harness (5199/9121).
export default defineConfig({
  root: '/Users/indo/code/project/omon-gateway/web',
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: 5399,
    strictPort: true,
    fs: { allow: ['/Users/indo/code/project/omon-gateway/web'] },
    proxy: {
      '/api': { target: 'http://127.0.0.1:9231', ws: true },
    },
  },
})
