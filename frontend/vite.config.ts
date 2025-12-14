import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import checker from 'vite-plugin-checker';

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte(),
  checker({
    typescript: true,
    overlay: {
      initialIsOpen: false,
    },
  }),
  ],

  build: {
    outDir: '../static',
    emptyOutDir: true,
    target: 'es2020',
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
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
    proxy: {
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
