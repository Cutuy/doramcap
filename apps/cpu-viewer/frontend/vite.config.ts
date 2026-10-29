import { defineConfig } from 'vite';
export default defineConfig({ base: './', server: { proxy: { '/clock': 'http://127.0.0.1:8080', '/stream': { target: 'ws://127.0.0.1:8080', ws: true } } } });
