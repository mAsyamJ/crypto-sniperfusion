import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// The frontend speaks ONLY to the local Rust API (same-origin via proxy);
// it makes no other network calls (ADR-0013, REQUIREMENTS NFR-1).
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': { target: 'http://127.0.0.1:7433', changeOrigin: false },
    },
  },
  preview: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': { target: 'http://127.0.0.1:7433', changeOrigin: false },
    },
  },
  build: { outDir: 'dist', sourcemap: false },
});
