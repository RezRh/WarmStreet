import { defineConfig } from 'vite';
import solidPlugin from 'vite-plugin-solid';
import path from 'path';

export default defineConfig({
  plugins: [solidPlugin()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  // Prevent Vite from obfuscating symbols needed for Tauri IPC
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  // Optimize for mobile WebViews - native-level startup speeds
  build: {
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  envPrefix: ['VITE_', 'TAURI_'],
});
