import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // Bundle-size analysis: `npm run build:analyze` (vite --mode analyze)
  // emits a dependency/asset visualization into ./dist/analyze.html plus a
  // machine-readable report (stats.json) that scripts/bundle-size.mjs and CI
  // use to enforce a size budget (see apps/desktop/scripts/bundle-size.mjs).
  const { visualizer } = await import("rollup-plugin-visualizer").catch((e) => {
    throw new Error("rollup-plugin-visualizer missing; run npm install", { cause: e });
  });
  const analyze =
    mode === "analyze"
      ? [
          visualizer({
            filename: "dist/analyze.html",
            gzipSize: true,
            template: "sunburst",
          }),
          visualizer({
            filename: "dist/stats.json",
            gzipSize: true,
            template: "raw-data",
          }),
        ]
      : [];

  return {
    plugins: [react(), ...analyze],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  };
});
