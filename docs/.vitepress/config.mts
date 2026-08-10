import { defineConfig, type DefaultTheme } from 'vitepress'
import { getReleases } from './utils/releases'

const REPO = 'https://github.com/IvanHanloth/Boss-Key'

// 更新日志侧边栏在构建时从 GitHub Releases 动态生成，见 changelog/[version].paths.ts。
// 三种语言共用同一份 Release 列表，仅前缀与「更早版本」一项的文案不同。
function changelogSidebar(
  releases: Awaited<ReturnType<typeof getReleases>>,
  prefix: string,
  legacyText: string,
): DefaultTheme.SidebarItem[] {
  const items = releases.map((release) => ({
    text: release.name,
    link: `${prefix}/changelog/${release.tag_name}`,
  }))
  // 迁移到 GitHub Releases 之前的版本，没有 Release 记录可查，归档在单独的静态页面。
  items.push({ text: legacyText, link: `${prefix}/changelog/legacy` })
  return items
}

// https://vitepress.dev/reference/site-config
export default defineConfig(async () => {
  const releases = await getReleases()

  return {
    // 动态路由的 frontmatter 是静态 YAML，不会解析 {{ $params.* }}；
    // 这里用 params 直接改写页面标题（<title> 标签），配合 changelog/[version].md 里的 H1 使用。
    transformPageData(pageData) {
      if (pageData.params?.title) {
        pageData.title = String(pageData.params.title)
      }
    },
    title: 'ZoneDeck',

    base: '/',
    lastUpdated: true,
    cleanUrls: true,
    ignoreDeadLinks: true,

    head: [
      ['link', { rel: 'icon', href: '/static/logo.svg', type: 'image/svg+xml' }],
      ['link', { rel: 'icon', href: '/static/icon.png', sizes: 'any' }],
    ],

    themeConfig: {
      logo: '/static/logo.svg',
      socialLinks: [{ icon: 'github', link: REPO }],
      search: { provider: 'local' },
    },

    locales: {
      root: {
        label: '简体中文',
        lang: 'zh-CN',
        description: '老板来了？快用 ZoneDeck 老板键一键隐藏静音当前窗口！上班摸鱼必备神器',
        themeConfig: {
          nav: [
            { text: '首页', link: '/' },
            { text: '使用文档', link: '/guide/', activeMatch: '/guide/' },
            { text: '开发文档', link: '/dev/', activeMatch: '/dev/' },
            { text: '更新日志', link: '/changelog/', activeMatch: '/changelog/' },
            {
              text: '下载',
              items: [
                { text: 'Release 下载页', link: `${REPO}/releases` },
                { text: '最新版本', link: `${REPO}/releases/latest` },
              ],
            },
          ],
          sidebar: {
            '/guide/': [
              {
                text: '开始使用',
                items: [
                  { text: '简介', link: '/guide/' },
                  { text: '安装与版本选择', link: '/guide/installation' },
                  { text: '快速上手', link: '/guide/getting-started' },
                ],
              },
              {
                text: '核心功能',
                items: [
                  { text: '绑定窗口与进程', link: '/guide/binding' },
                  { text: '热键与鼠标手势', link: '/guide/hotkeys' },
                  { text: '进程冻结', link: '/guide/freeze' },
                  { text: '其他选项', link: '/guide/options' },
                  { text: '提示设置', link: '/guide/notifications' },
                  { text: '开机自启', link: '/guide/autostart' },
                ],
              },
              {
                text: '维护与排障',
                items: [
                  { text: '检查更新与反馈', link: '/guide/update' },
                  { text: '窗口恢复与崩溃自愈', link: '/guide/recovery' },
                  { text: '常见问题', link: '/guide/faq' },
                ],
              },
            ],
            '/dev/': [
              {
                text: '概览',
                items: [
                  { text: '开发文档导览', link: '/dev/' },
                  { text: '本地运行', link: '/dev/getting-started' },
                ],
              },
              {
                text: '参与贡献',
                items: [
                  { text: '贡献指南', link: '/dev/contributing' },
                  { text: '项目管理策略', link: '/dev/project-management' },
                ],
              },
              {
                text: '深入项目',
                items: [
                  { text: '系统架构', link: '/dev/architecture' },
                  { text: '前端与配置界面', link: '/dev/frontend' },
                  { text: '测试策略', link: '/dev/testing' },
                  { text: '打包与发布', link: '/dev/release' },
                  { text: '文档站维护', link: '/dev/docs-site' },
                ],
              },
              {
                text: '参考',
                items: [
                  { text: '配置文件字段', link: '/dev/config-reference' },
                  { text: 'IPC 协议', link: '/dev/ipc-protocol' },
                ],
              },
            ],
            '/changelog/': [
              { text: '版本', items: changelogSidebar(releases, '', '更早版本') },
            ],
          },
          editLink: {
            pattern: `${REPO}/edit/main/docs/:path`,
            text: '在 GitHub 上编辑此页',
          },
          footer: {
            message: '基于 MIT 许可发布',
            copyright:
              'Copyright © 2022-present <a href="https://www.ivan-hanloth.cn">IvanHanloth</a> All Rights Reserved.',
          },
          docFooter: { prev: '上一页', next: '下一页' },
          outline: { label: '页面导航', level: [2, 3] },
          lastUpdated: { text: '最后更新于' },
          returnToTopLabel: '回到顶部',
          sidebarMenuLabel: '菜单',
          darkModeSwitchLabel: '主题',
          lightModeSwitchTitle: '切换到浅色模式',
          darkModeSwitchTitle: '切换到深色模式',
          langMenuLabel: '切换语言',
        },
      },

      en: {
        label: 'English',
        lang: 'en',
        link: '/en/',
        description:
          'Boss coming? Hide, mute and freeze the current window with a single key — ZoneDeck.',
        themeConfig: {
          nav: [
            { text: 'Home', link: '/en/' },
            { text: 'Guide', link: '/en/guide/', activeMatch: '/en/guide/' },
            { text: 'Development', link: '/en/dev/', activeMatch: '/en/dev/' },
            { text: 'Changelog', link: '/en/changelog/', activeMatch: '/en/changelog/' },
            {
              text: 'Download',
              items: [
                { text: 'Releases', link: `${REPO}/releases` },
                { text: 'Latest release', link: `${REPO}/releases/latest` },
              ],
            },
          ],
          sidebar: {
            '/en/guide/': [
              {
                text: 'Getting started',
                items: [
                  { text: 'Introduction', link: '/en/guide/' },
                  { text: 'Installation & editions', link: '/en/guide/installation' },
                  { text: 'Quick start', link: '/en/guide/getting-started' },
                ],
              },
              {
                text: 'Features',
                items: [
                  { text: 'Binding windows & processes', link: '/en/guide/binding' },
                  { text: 'Hotkeys & mouse gestures', link: '/en/guide/hotkeys' },
                  { text: 'Process freezing', link: '/en/guide/freeze' },
                  { text: 'Other options', link: '/en/guide/options' },
                  { text: 'Alerts', link: '/en/guide/notifications' },
                  { text: 'Start with Windows', link: '/en/guide/autostart' },
                ],
              },
              {
                text: 'Maintenance & troubleshooting',
                items: [
                  { text: 'Updates & feedback', link: '/en/guide/update' },
                  { text: 'Window recovery & crash self-healing', link: '/en/guide/recovery' },
                  { text: 'FAQ', link: '/en/guide/faq' },
                ],
              },
            ],
            '/en/dev/': [
              {
                text: 'Overview',
                items: [
                  { text: 'Development docs', link: '/en/dev/' },
                  { text: 'Running locally', link: '/en/dev/getting-started' },
                ],
              },
              {
                text: 'Contributing',
                items: [
                  { text: 'Contribution guide', link: '/en/dev/contributing' },
                  { text: 'Project management', link: '/en/dev/project-management' },
                ],
              },
              {
                text: 'Deep dive',
                items: [
                  { text: 'Architecture', link: '/en/dev/architecture' },
                  { text: 'Frontend & settings UI', link: '/en/dev/frontend' },
                  { text: 'Testing strategy', link: '/en/dev/testing' },
                  { text: 'Packaging & releasing', link: '/en/dev/release' },
                  { text: 'Docs site', link: '/en/dev/docs-site' },
                ],
              },
              {
                text: 'Reference',
                items: [
                  { text: 'Configuration fields', link: '/en/dev/config-reference' },
                  { text: 'IPC protocol', link: '/en/dev/ipc-protocol' },
                ],
              },
            ],
            '/en/changelog/': [
              { text: 'Versions', items: changelogSidebar(releases, '/en', 'Earlier versions') },
            ],
          },
          editLink: {
            pattern: `${REPO}/edit/main/docs/:path`,
            text: 'Edit this page on GitHub',
          },
          footer: {
            message: 'Released under the MIT License',
            copyright:
              'Copyright © 2022-present <a href="https://www.ivan-hanloth.cn">IvanHanloth</a> All Rights Reserved.',
          },
        },
      },

      'zh-tw': {
        label: '繁體中文',
        lang: 'zh-TW',
        link: '/zh-tw/',
        description: '老闆來了？快用 ZoneDeck 老闆鍵一鍵隱藏靜音目前視窗！上班摸魚必備神器',
        themeConfig: {
          nav: [
            { text: '首頁', link: '/zh-tw/' },
            { text: '使用說明', link: '/zh-tw/guide/', activeMatch: '/zh-tw/guide/' },
            { text: '開發文件', link: '/zh-tw/dev/', activeMatch: '/zh-tw/dev/' },
            { text: '更新日誌', link: '/zh-tw/changelog/', activeMatch: '/zh-tw/changelog/' },
            {
              text: '下載',
              items: [
                { text: 'Release 下載頁', link: `${REPO}/releases` },
                { text: '最新版本', link: `${REPO}/releases/latest` },
              ],
            },
          ],
          sidebar: {
            '/zh-tw/guide/': [
              {
                text: '開始使用',
                items: [
                  { text: '簡介', link: '/zh-tw/guide/' },
                  { text: '安裝與版本選擇', link: '/zh-tw/guide/installation' },
                  { text: '快速上手', link: '/zh-tw/guide/getting-started' },
                ],
              },
              {
                text: '核心功能',
                items: [
                  { text: '綁定視窗與程序', link: '/zh-tw/guide/binding' },
                  { text: '快速鍵與滑鼠手勢', link: '/zh-tw/guide/hotkeys' },
                  { text: '程序凍結', link: '/zh-tw/guide/freeze' },
                  { text: '其他選項', link: '/zh-tw/guide/options' },
                  { text: '提示設定', link: '/zh-tw/guide/notifications' },
                  { text: '開機自動啟動', link: '/zh-tw/guide/autostart' },
                ],
              },
              {
                text: '維護與排障',
                items: [
                  { text: '檢查更新與意見回饋', link: '/zh-tw/guide/update' },
                  { text: '視窗復原與當機自癒', link: '/zh-tw/guide/recovery' },
                  { text: '常見問題', link: '/zh-tw/guide/faq' },
                ],
              },
            ],
            '/zh-tw/dev/': [
              {
                text: '概覽',
                items: [
                  { text: '開發文件導覽', link: '/zh-tw/dev/' },
                  { text: '本機執行', link: '/zh-tw/dev/getting-started' },
                ],
              },
              {
                text: '參與貢獻',
                items: [
                  { text: '貢獻指南', link: '/zh-tw/dev/contributing' },
                  { text: '專案管理策略', link: '/zh-tw/dev/project-management' },
                ],
              },
              {
                text: '深入專案',
                items: [
                  { text: '系統架構', link: '/zh-tw/dev/architecture' },
                  { text: '前端與設定介面', link: '/zh-tw/dev/frontend' },
                  { text: '測試策略', link: '/zh-tw/dev/testing' },
                  { text: '打包與發布', link: '/zh-tw/dev/release' },
                  { text: '文件站維護', link: '/zh-tw/dev/docs-site' },
                ],
              },
              {
                text: '參考',
                items: [
                  { text: '設定檔欄位', link: '/zh-tw/dev/config-reference' },
                  { text: 'IPC 協定', link: '/zh-tw/dev/ipc-protocol' },
                ],
              },
            ],
            '/zh-tw/changelog/': [
              { text: '版本', items: changelogSidebar(releases, '/zh-tw', '更早版本') },
            ],
          },
          editLink: {
            pattern: `${REPO}/edit/main/docs/:path`,
            text: '在 GitHub 上編輯此頁',
          },
          footer: {
            message: '基於 MIT 授權條款發布',
            copyright:
              'Copyright © 2022-present <a href="https://www.ivan-hanloth.cn">IvanHanloth</a> All Rights Reserved.',
          },
          docFooter: { prev: '上一頁', next: '下一頁' },
          outline: { label: '本頁導覽', level: [2, 3] },
          lastUpdated: { text: '最後更新於' },
          returnToTopLabel: '回到頂端',
          sidebarMenuLabel: '選單',
          darkModeSwitchLabel: '佈景主題',
          lightModeSwitchTitle: '切換至淺色模式',
          darkModeSwitchTitle: '切換至深色模式',
          langMenuLabel: '切換語言',
        },
      },
    },
  }
})
