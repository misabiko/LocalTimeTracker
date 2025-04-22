import {defineConfig} from 'vite';
import {svelte} from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
    clearScreen: false,
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
            ignored: ['**/src-tauri/**'],
        }
    },
    envPrefix: ['VITE_', 'TAURI_ENV_*'],
    root: 'src',
    build: {
        outDir: '../dist',
        emptyOutDir: true,
        target: 'esnext',
        minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
        sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
    plugins: [svelte()],
});
