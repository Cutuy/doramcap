import { defineConfig } from "vite";
export default defineConfig({ base: "./", server: { proxy: { "/api": "http://127.0.0.1:8090", "/stream": { target: "ws://127.0.0.1:8090", ws: true } } } });
