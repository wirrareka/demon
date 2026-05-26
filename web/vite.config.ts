import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The daemon serves the API over (WireGuard) HTTP/mTLS. In dev we proxy to a local
// daemon so the browser app is same-origin (no CORS needed — the daemon's CORS
// allowlist stays strict). Point DEMON_DEV_API at your running daemon.
const target = process.env.DEMON_DEV_API ?? "http://127.0.0.1:8791";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": { target, changeOrigin: true, ws: true },
      "/auth": { target, changeOrigin: true },
      "/health": { target, changeOrigin: true },
      "/version": { target, changeOrigin: true },
    },
  },
});
