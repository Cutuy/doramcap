import { copyFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
// Clean URLs need absolute assets. Build for a subpath with `npm run build -- --base /doramcap/`.
export default defineConfig({
  base: '/',
  plugins: [{
    // GitHub Pages serves 404.html for unknown paths, so deep links still load the app.
    name: 'spa-404',
    writeBundle(options) { copyFileSync(resolve(options.dir!, 'index.html'), resolve(options.dir!, '404.html')); },
  }],
  // React libraries ship "use client" directives that Rollup reports and drops; they are harmless in this SPA.
  build: { rollupOptions: { onwarn(warning, warn) { if (warning.code !== 'MODULE_LEVEL_DIRECTIVE') warn(warning); } } },
});
