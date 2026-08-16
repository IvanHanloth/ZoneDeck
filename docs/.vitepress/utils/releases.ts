// 编译期获取 GitHub Releases，供更新日志页面（changelog/[version].md）与
// 侧边栏（config.mts）共用。用模块级 Promise 缓存，同一次构建里两处调用只请求一次。
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

export interface ReleaseAsset {
  name: string
  browser_download_url: string
}

export interface Release {
  tag_name: string
  name: string
  published_at: string
  body: string
  html_url: string
  assets: ReleaseAsset[]
}

const REPO = process.env.GITHUB_REPOSITORY ?? 'IvanHanloth/ZoneDeck'
// 网络不可用（例如本地无 token 触发限流）时，退回仓库里已提交的快照，保证构建不失败。
const FALLBACK_PATH = fileURLToPath(new URL('../../public/releases.json', import.meta.url))

let cache: Promise<Release[]> | undefined

export function getReleases(): Promise<Release[]> {
  if (!cache) cache = fetchReleases()
  return cache
}

async function fetchReleases(): Promise<Release[]> {
  try {
    const headers: Record<string, string> = { Accept: 'application/vnd.github.v3+json' }
    const token = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN
    if (token) headers.Authorization = `Bearer ${token}`

    const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`, { headers })
    if (!res.ok) throw new Error(`GitHub API ${res.status}`)

    const data = (await res.json()) as any[]
    return sortByDate(
      data
        .filter((r) => !r.draft)
        .map((r) => ({
          tag_name: r.tag_name,
          name: r.name || r.tag_name,
          published_at: r.published_at,
          body: r.body ?? '',
          html_url: r.html_url,
          assets: (r.assets ?? []).map((a: any) => ({
            name: a.name,
            browser_download_url: a.browser_download_url,
          })),
        }))
    )
  } catch (err) {
    console.warn(`[changelog] 获取 GitHub Releases 失败，改用本地快照: ${(err as Error).message}`)
    const raw = JSON.parse(readFileSync(FALLBACK_PATH, 'utf-8')) as any[]
    return sortByDate(
      raw.map((r) => ({
        tag_name: r.tag_name,
        name: r.tag_name,
        published_at: r.published_at,
        body: r.body ?? '',
        html_url: `https://github.com/${REPO}/releases/tag/${r.tag_name}`,
        assets: r.assets ?? [],
      }))
    )
  }
}

function sortByDate(list: Release[]): Release[] {
  return [...list].sort((a, b) => new Date(b.published_at).getTime() - new Date(a.published_at).getTime())
}
