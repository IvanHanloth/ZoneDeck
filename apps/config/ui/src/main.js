import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { win } from "./lib/ipc.js";
import { afterFirstPaint } from "./lib/splash.js";
import { applyTheme, loadPreference } from "./lib/theme.js";

// 挂载前先应用主题，避免启动屏配色跳变。
applyTheme(loadPreference());

const app = mount(App, { target: document.getElementById("app") });

// 首帧绘制后再显示窗口。
afterFirstPaint(() => win.show());

export default app;
