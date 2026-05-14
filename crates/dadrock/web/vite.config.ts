import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: process.env.DADR_PREVIEW_URL ?? 'http://localhost:8765',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: true,
  },
});
