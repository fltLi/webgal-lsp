import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
// @ts-expect-error type error without @types/node package
import process from 'node:process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, URL } from 'node:url';
const host = process.env.TAURI_DEV_HOST;

/** splash.html 里的图标占位符, 由 `inlineSplashIcon` 替换为 public/icon.svg 的内容 */
const SPLASH_ICON_TOKEN = '<!-- inline:icon.svg -->';

/*
 * 开屏图标必须是内联 SVG: `<img>` 会多一次资源请求, 首帧就会出现"文字已经画好、图标还没到"。
 * 但同一份图标在标题栏 / 开始界面 / 设置「关于」里都是 public/icon.svg, 不该为了开屏再存一份副本,
 * 所以这里在开发与构建时把它注入 splash.html 的占位符 —— 图形始终只有一个来源。
 */
function inlineSplashIcon(): Plugin {
  let iconPath = '';
  let splashSource = '';
  let splashBuilt = '';

  const inject = (html: string) => {
    if (!html.includes(SPLASH_ICON_TOKEN)) return html;
    return html.replace(SPLASH_ICON_TOKEN, readFileSync(iconPath, 'utf8').trim());
  };

  return {
    name: 'webgal-ink:inline-splash-icon',
    configResolved(config) {
      const publicDir = typeof config.publicDir === 'string' ? config.publicDir : resolve(config.root, 'public');
      iconPath = resolve(publicDir, 'icon.svg');
      splashSource = resolve(publicDir, 'splash.html');
      splashBuilt = resolve(config.root, config.build.outDir, 'splash.html');
    },
    // 开发: public/ 下的 splash.html 由静态中间件原样吐出, 这里抢在它之前注入
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if ((req.url ?? '').split('?')[0] !== '/splash.html') return next();
        try {
          res.setHeader('Content-Type', 'text/html; charset=utf-8');
          res.end(inject(readFileSync(splashSource, 'utf8')));
        } catch (error) {
          next(error as Error);
        }
      });
    },
    // 构建: public/ 是在 writeBundle 阶段才被拷贝的, 用 closeBundle 保证文件已经就位
    closeBundle() {
      if (!existsSync(splashBuilt)) return;
      writeFileSync(splashBuilt, inject(readFileSync(splashBuilt, 'utf8')));
    },
  };
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [inlineSplashIcon(), react()],
  resolve: {
    alias: {
      'webgal-novel-preprocess': fileURLToPath(
        new URL('../packages/webgal-novel-preprocess/src/index.ts', import.meta.url)
      ),
    },
  },

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
