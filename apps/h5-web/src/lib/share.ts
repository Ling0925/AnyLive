/**
 * Pure share/query helpers for the H5 watch page.
 * Query param: `?room=<UUID>`
 */

const ROOM_PARAM = 'room'

/** Extract `room` query param from a search string (`?room=...` or full URL search). */
export function readRoomFromQuery(search: string): string {
  const q = search.startsWith('?') ? search.slice(1) : search
  const params = new URLSearchParams(q)
  return (params.get(ROOM_PARAM) ?? '').trim()
}

/**
 * Build a shareable URL that deep-links to a room.
 * `href` may be a full URL or a path; room query is set/replaced.
 */
export function buildShareUrl(href: string, roomId: string): string {
  const id = roomId.trim()
  // Prefer URL API when base is absolute; fall back for relative paths in tests/SSR.
  try {
    const url = new URL(href)
    if (id) {
      url.searchParams.set(ROOM_PARAM, id)
    } else {
      url.searchParams.delete(ROOM_PARAM)
    }
    return url.toString()
  } catch {
    const [pathAndQuery, hash = ''] = href.split('#')
    const [path, query = ''] = pathAndQuery.split('?')
    const params = new URLSearchParams(query)
    if (id) {
      params.set(ROOM_PARAM, id)
    } else {
      params.delete(ROOM_PARAM)
    }
    const qs = params.toString()
    const base = qs ? `${path}?${qs}` : path
    return hash ? `${base}#${hash}` : base
  }
}

/** True when the room is no longer watchable (idle or closed). */
export function isRoomEnded(status: string): boolean {
  return status === 'closed' || status === 'idle'
}
