import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";

const DASHBOARD_PROXY_TARGET =
  process.env.DASHBOARD_API_PROXY ?? "http://127.0.0.1:3030";

// https://vitejs.dev/config/
export default defineConfig(() => ({
  base: "/static/",
  server: {
    host: "::",
    port: 8080,
    proxy: {
      "/api": {
        target: DASHBOARD_PROXY_TARGET,
        changeOrigin: true,
        secure: false,
      },
    },
  },
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    outDir: "../static",
    emptyOutDir: true,
    assetsDir: "assets",
    sourcemap: false,
    cssCodeSplit: false,
  },
}));
