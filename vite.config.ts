import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: { proxy: { '/api': 'http://127.0.0.1:3000' } },
  test: {
    include: ['frontend/**/*.test.ts'],
    environment: 'jsdom',
    setupFiles: ['frontend/test-setup.ts'],
    server: { deps: { inline: ['svelte'] } },
  },
  resolve: { conditions: ['browser'] },
});
