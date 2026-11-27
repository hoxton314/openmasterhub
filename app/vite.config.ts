import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const page = (file: string) => fileURLToPath(new URL(file, import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5174, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  // Two windows: the app (index.html) and the switch toast near the tray (overlay.html).
  build: { rollupOptions: { input: { main: page('index.html'), overlay: page('overlay.html') } } },
});
