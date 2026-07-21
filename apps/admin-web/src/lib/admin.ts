export function adminTitle(env: string): string {
  return env === 'prod' ? 'AnyLive Admin' : `AnyLive Admin (${env})`
}

export function canAccessModule(role: string, module: string): boolean {
  if (role === 'admin') return true
  if (role === 'moderator') {
    return ['rooms', 'reports', 'users'].includes(module)
  }
  return false
}
