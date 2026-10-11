import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import process from 'node:process';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [sveltekit()],
  /*
   * 这里**不再**写 `resolve.alias`。
   *
   * `webgal-novel-preprocess` 的别名统一由 `svelte.config.js` 的 `kit.alias` 提供 ——
   * SvelteKit 会把它同时喂给 Vite 与自动生成的 `.svelte-kit/tsconfig.json`, 于是打包期与
   * 类型检查用的是同一个映射。两处各写一份迟早漂移, 而且 tsconfig 里那份会让 `vite dev`
   * 报 "interferes with SvelteKit's auto-generated tsconfig.json"。
   */

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
