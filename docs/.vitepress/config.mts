import { defineConfig } from 'vitepress'
import { getReleases } from './utils/releases'

// https://vitepress.dev/reference/site-config
export default defineConfig(async () => {
  // 更新日志侧边栏在构建时从 GitHub Releases 动态生成，见 changelog/[version].paths.ts
  const releases = await getReleases()
  const changelogSidebar = releases.map((release) => ({
    text: release.name,
    link: `/changelog/${release.tag_name}`,
  }))
  // 迁移到 GitHub Releases 之前的版本，没有 Release 记录可查，归档在单独的静态页面。
  changelogSidebar.push({ text: '更早版本', link: '/changelog/legacy' })

  return {
    // 动态路由的 frontmatter 是静态 YAML，不会解析 {{ $params.* }}；
    // 这里用 params 直接改写页面标题（<title> 标签），配合 changelog/[version].md 里的 H1 使用。
    transformPageData(pageData) {
      if (pageData.params?.title) {
        pageData.title = String(pageData.params.title)
      }
    },
    lang: 'zh-CN',
    title: 'Boss Key',
    description: '老板来了？快用 Boss Key 老板键一键隐藏静音当前窗口！上班摸鱼必备神器',

    base: '/',
    lastUpdated: true,
    cleanUrls: true,
    ignoreDeadLinks: true,

    head: [
      ['link', { rel: 'icon', href: '/static/icon.png' }],
    ],

    themeConfig: {
      // https://vitepress.dev/reference/default-theme-config
      logo: '/static/icon.png',

      nav: [
        { text: '首页', link: '/' },
        { text: '使用文档', link: '/guide/', activeMatch: '/guide/' },
        { text: '开发文档', link: '/dev/', activeMatch: '/dev/' },
        { text: '更新日志', link: '/changelog/', activeMatch: '/changelog/' },
        {
          text: '下载',
          items: [
            { text: 'Release 下载页', link: 'https://github.com/IvanHanloth/Boss-Key/releases' },
            { text: '最新版本', link: 'https://github.com/IvanHanloth/Boss-Key/releases/latest' },
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
              { text: '通知设置', link: '/guide/notifications' },
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
          {
            text: '版本',
            items: changelogSidebar,
          },
        ],
      },

      socialLinks: [
        { icon: 'github', link: 'https://github.com/IvanHanloth/Boss-Key' },
      ],

      editLink: {
        pattern: 'https://github.com/IvanHanloth/Boss-Key/edit/main/docs/:path',
        text: '在 GitHub 上编辑此页',
      },

      footer: {
        message: '基于 MIT 许可发布',
        copyright: 'Copyright © 2022-present <a href="https://www.ivan-hanloth.cn">IvanHanloth</a> All Rights Reserved.',
      },

      search: {
        provider: 'local',
      },

      docFooter: {
        prev: '上一页',
        next: '下一页',
      },
      outline: {
        label: '页面导航',
        level: [2, 3],
      },
      lastUpdated: {
        text: '最后更新于',
      },
      returnToTopLabel: '回到顶部',
      sidebarMenuLabel: '菜单',
      darkModeSwitchLabel: '主题',
      lightModeSwitchTitle: '切换到浅色模式',
      darkModeSwitchTitle: '切换到深色模式',
    },
  }
})
