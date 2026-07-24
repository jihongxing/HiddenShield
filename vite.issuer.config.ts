import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  root: "src-issuer",
  plugins: [vue()],
  server: {
    host: "127.0.0.1",
    port: 1421,
    strictPort: true,
  },
  build: {
    outDir: "../dist-issuer",
    emptyOutDir: true,
  },
});
