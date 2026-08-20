import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { invoke } from "./lib/ipc.js";
import { applyTheme, loadPreference } from "./lib/theme.js";

// 问后端窗口材质：Mica 可用时 body 留透明让 DWM 画，不可用（Win10、浏览器预览）
// 则维持 index.html 铺的那层不透明底色，否则会直接透出桌面。
// 不等它 —— 一次 IPC 往返不该挡住挂载。
invoke("backdrop_kind")
  .then((kind) => document.documentElement.classList.add(`backdrop-${kind}`))
  .catch(() => document.documentElement.classList.add("backdrop-solid"));

// 挂载前先应用主题，避免配色跳变。
applyTheme(loadPreference());

const app = mount(App, { target: document.getElementById("app") });

export default app;
