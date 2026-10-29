import { defineConfig } from "vite";
export default defineConfig({base:"./",build:{rollupOptions:{input:{index:"index.html",snapshot:"src/snapshot.ts"},preserveEntrySignatures:"strict",output:{entryFileNames:"[name].js"}}},server:{port:5175,proxy:{"/api":{target:"http://127.0.0.1:8091",ws:true}}}});
