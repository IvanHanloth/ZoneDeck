// https://vitepress.dev/guide/custom-theme
import { h } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
// Lucide 图标（Vue 版），与配置界面所用图标同源，确保文档里显示的图标与应用一致。
import { Shield, Play, RotateCw, Power, ScrollText, Check } from 'lucide-vue-next'
import StatusBar from './components/StatusBar.vue'
import './style.css'

export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      // https://vitepress.dev/guide/extending-default-theme#layout-slots
    })
  },
  enhanceApp({ app, router, siteData }) {
    // 复刻配置界面状态栏的组件，可在 Markdown 中直接以 <StatusBar /> 使用。
    app.component('StatusBar', StatusBar)
    // 全局注册状态栏用到的 Lucide 图标，Markdown 里可内联使用，例如 <Shield :size="14" />。
    app.component('Shield', Shield)
    app.component('Play', Play)
    app.component('RotateCw', RotateCw)
    app.component('Power', Power)
    app.component('ScrollText', ScrollText)
    app.component('Check', Check)
  }
} satisfies Theme
