import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import checker from 'vite-plugin-checker';
import { mockBackend } from './mock/plugin';

// Set VITE_MOCK=1 (see `npm run dev:mock`) to serve a fake backend locally
// instead of proxying to the Rust server on :8080.
const useMock = !!process.env.VITE_MOCK;

// https://vite.dev/config/
export default defineConfig({
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
    outDir: '../static',
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
