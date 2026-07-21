export function buildPlayUrl(base: string, stream: string): string {
  const b = base.replace(/\/$/, '')
  const s = stream.replace(/^\//, '')
  return `${b}/${s}.m3u8`
}

export function isLiveStatus(status: string): boolean {
  return status === 'live'
}
