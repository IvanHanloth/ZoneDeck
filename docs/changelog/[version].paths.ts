import { getReleases } from '../.vitepress/utils/releases'

export default {
  async paths() {
    const releases = await getReleases()
    return releases.map((release) => ({
      params: {
        version: release.tag_name,
        title: release.name,
        publishedAt: release.published_at,
        htmlUrl: release.html_url,
        assets: release.assets,
      },
      content: release.body.trim() || '_此版本没有更新说明。_',
    }))
  },
}
