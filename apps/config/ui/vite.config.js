import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// 构建产物输出到 ../dist，由 tauri.conf.json 的 frontendDist 内嵌进配置程序。
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "chrome110", // WebView2 常青版本，放心用现代语法
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.js"],
  },
});
