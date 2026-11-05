import { defineConfig } from 'vite';
const REPOSITORY_ROOT = '../../..';
// Relative assets allow the same dist directory to be served at / or /doramcap/.
// Console source is linked from the repository, so it shares this app's React.
export default defineConfig({ base: './', resolve: { dedupe: ['react', 'react-dom'] }, server: { fs: { allow: [REPOSITORY_ROOT] } } });
