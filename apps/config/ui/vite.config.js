import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import Icons from "unplugin-icons/vite";

// 构建产物输出到 ../dist，由 tauri.conf.json 的 frontendDist 内嵌进配置程序。
// 图标：unplugin-icons 在构建期把用到的 SVG（来自本地 @iconify-json/lucide）内联进产物，
// tree-shaking 只打包引用到的图标，运行期零网络请求（autoInstall 关闭，绝不联网抓取）。
export default defineConfig({
  plugins: [svelte(), Icons({ compiler: "svelte", autoInstall: false })],
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
