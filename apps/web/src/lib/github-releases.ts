export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    /** The Mac app's zip. */
    mac: string | null
  }
}

interface GhAsset {
  name: string
  browser_download_url: string
}

interface GhRelease {
  tag_name: string
  name?: string
  published_at: string
  html_url: string
  assets: GhAsset[]
}

const REPO = 'ZainW/mdow'
/** The Mac app: `Mdow-<version>.zip`, and the same under a fixed name. */
const MAC_APP_ALIAS = 'mdow.zip'
const MAC_APP_VERSIONED = /^mdow-\d+\.\d+\.\d+(-[0-9a-z.]+)?\.zip$/

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  let mac: string | null = null
  for (const asset of release.assets) {
    const normalized = asset.name.toLowerCase()
    // The versioned name is the one the release notes and update feed point at.
    if (MAC_APP_VERSIONED.test(normalized)) mac = asset.browser_download_url
    else if (normalized === MAC_APP_ALIAS) mac ??= asset.browser_download_url
  }

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac },
  }
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'User-Agent': 'mdow-website',
      },
    })
    if (!res.ok) return null
    const json = (await res.json()) as GhRelease
    return parseRelease(json)
  } catch {
    return null
  }
}
