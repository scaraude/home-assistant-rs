import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import checker from 'vite-plugin-checker';
import { mockBackend } from './mock/plugin';

// Set VITE_MOCK=1 (see `npm run dev:mock`) to serve a fake backend locally
// instead of proxying to the Rust server on :8080.
const useMock = !!process.env.VITE_MOCK;

// Set VITE_MOCK_BUILD=1 (see `npm run build:demo`) to produce a static, backend-
// less bundle for the online demo: the in-browser mock (mock/browser.ts) is
// pulled in at runtime and the assets are served under a GitHub Pages base path.
const demoBuild = !!process.env.VITE_MOCK_BUILD;
// GitHub Pages serves a project site under /<repo>/. Override with VITE_BASE if
// hosting elsewhere (e.g. a custom domain or a different repo name).
const base = demoBuild ? process.env.VITE_BASE ?? '/home-automation-rs/' : '/';

// https://vite.dev/config/
export default defineConfig({
  base,

  plugins: [
    svelte(),
    checker({
      typescript: true,
      overlay: {
        initialIsOpen: false,
      },
    }),
    ...(useMock ? [mockBackend()] : []),
  ],

  build: {
    // Keep the demo bundle out of ../static (the bundle the Rust server ships).
    outDir: demoBuild ? 'dist-demo' : '../static',
    emptyOutDir: true,
    target: 'es2020',
    minify: 'terser',
    terserOptions: {
      compress: {
        // drop_console: true,
        drop_debugger: true,
      },
    },
    rollupOptions: {
      output: {
        manualChunks: undefined,
      },
    },
  },

  server: {
    // In mock mode the plugin owns /api and /ws, so the proxy must stay off.
    proxy: useMock
      ? undefined
      : {
          '/api': {
            target: 'http://localhost:8080',
            changeOrigin: true,
          },
          '/ws': {
            target: 'ws://localhost:8080',
            changeOrigin: true,
            ws: true,
          },
        },
  },
})
