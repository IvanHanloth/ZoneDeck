import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { win } from "./lib/ipc.js";
import { afterFirstPaint } from "./lib/splash.js";
import { applyTheme, loadPreference } from "./lib/theme.js";

// 主题必须在挂载前落到 <html data-theme> 上，否则启动屏会先按系统配色画、再跳成用户偏好。
applyTheme(loadPreference());

const app = mount(App, { target: document.getElementById("app") });

// 窗口以 visible:false 启动；等启动屏真的画到屏幕上了再 show，用户就看不到白屏。
// 配置加载完由 App 把启动屏淡掉。
afterFirstPaint(() => win.show());

export default app;
