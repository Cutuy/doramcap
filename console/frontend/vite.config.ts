import { defineConfig } from 'vite';
// First browsers with CSS light-dark(); keeping it native lets the theme switch override the system scheme.
const LIGHT_DARK_BROWSERS = ['chrome123', 'edge123', 'firefox120', 'safari17.5'];
export default defineConfig({ build: { cssTarget: LIGHT_DARK_BROWSERS } });
