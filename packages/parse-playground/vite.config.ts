import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const wasmPkgRoot = path.resolve(repoRoot, 'crates', 'webgal-parse-wasm', 'pkg');

export default defineConfig({
    plugins: [react()],
    resolve: {
        alias: {
            'webgal-parse-wasm': wasmPkgRoot,
        },
    },
    server: {
        port: 5173,
        host: '0.0.0.0',
        fs: {
            allow: [repoRoot],
        },
    },
});
