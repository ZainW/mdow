import { closeSync, openSync, readSync, statSync } from 'node:fs'
import { imageSize as measure } from 'image-size'

const HEADER_BYTES = 512 * 1024
const cache = new Map<string, { mtime: number; size: ImageSize | null }>()

export interface ImageSize {
  width: number
  height: number
}

/** Pixel size of a local image, read from its header. Null when missing or unreadable. */
export function imageSize(path: string): ImageSize | null {
  let mtime: number
  try {
    mtime = statSync(path).mtimeMs
  } catch {
    return null
  }
  const cached = cache.get(path)
  if (cached && cached.mtime === mtime) return cached.size

  let size: ImageSize | null = null
  let fd: number | null = null
  try {
    fd = openSync(path, 'r')
    const buffer = Buffer.alloc(HEADER_BYTES)
    const read = readSync(fd, buffer, 0, HEADER_BYTES, 0)
    const result = measure(buffer.subarray(0, read))
    const rotated = (result.orientation ?? 1) >= 5
    if (result.width && result.height) {
      size = rotated
        ? { width: result.height, height: result.width }
        : { width: result.width, height: result.height }
    }
  } catch {
    size = null
  } finally {
    if (fd !== null) closeSync(fd)
  }
  cache.set(path, { mtime, size })
  return size
}

/** Pixel size of a `data:image/…` URL, base64 or percent-encoded. */
export function dataImageSize(url: string): ImageSize | null {
  const comma = url.indexOf(',')
  if (comma === -1) return null
  const meta = url.slice(0, comma)
  const payload = url.slice(comma + 1)
  try {
    const bytes = /;base64$/i.test(meta)
      ? Buffer.from(payload, 'base64')
      : Buffer.from(decodeURIComponent(payload))
    const result = measure(bytes)
    return result.width && result.height ? { width: result.width, height: result.height } : null
  } catch {
    return null
  }
}
