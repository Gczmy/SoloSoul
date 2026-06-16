import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  clearScreen: false,
  server: {
    port: Number(process.env.SOLOSOUL_VITE_PORT) || 1420,
    strictPort: true,
    host: host || false,
    hmr: {
      protocol: 'ws',
      host: host || 'localhost',
      port: Number(process.env.SOLOSOUL_VITE_HMR_PORT) || 1421,
    },
    watch: {
      ignored: ['**/src-tauri/**', '**/target/**'],
    },
  },
});
