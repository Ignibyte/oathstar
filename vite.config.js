import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  server: {
    host: host || "127.0.0.1",
    port: 5173,
    strictPort: true,
    // Same-origin in dev: proxy the game API to the local oathstar-server so the
    // browser client (and its EventSource) avoids cross-origin CORS. A Tauri
    // build instead points VITE_OATHSTAR_API at the loopback server directly.
    proxy: {
      "/command": "http://127.0.0.1:7878",
      "/state": "http://127.0.0.1:7878",
      "/events": { target: "http://127.0.0.1:7878", changeOrigin: true }
    },
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"]
    }
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2022",
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG)
  }
});
