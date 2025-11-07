import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./tests/frontend/setup.ts'],
    include: ['tests/frontend/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    exclude: ['node_modules', 'dist', 'src-tauri'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'tests/',
        'src-tauri/',
        '**/*.d.ts',
        '**/*.config.{js,ts}',
      ],
    },
  },
  resolve: {
    alias: {
      '@': '/src',
    },
  },
});
