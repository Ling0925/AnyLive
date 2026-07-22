/**
 * Pure API path builders + response parse helpers for H5 chat / gifts / wallet.
 */

/** Normalize API base URL (strip trailing slash). */
export function normalizeApiBase(base: string): string {
  return base.endsWith('/') ? base.slice(0, -1) : base
}

/** Join base URL with an absolute API path. */
export function apiUrl(base: string, path: string): string {
  const root = normalizeApiBase(base)
  const p = path.startsWith('/') ? path : `/${path}`
  return `${root}${p}`
}

function cleanId(id: string): string {
  return id.replace(/^\/+|\/+$/g, '')
}

// --- Path builders ---

export function otpSendPath(): string {
  return '/api/v1/auth/otp/send'
}

export function otpVerifyPath(): string {
  return '/api/v1/auth/otp/verify'
}

export function giftsPath(): string {
  return '/api/v1/gifts'
}

export function walletPath(): string {
  return '/api/v1/wallet'
}

export function walletTopupPath(): string {
  return '/api/v1/wallet/topups'
}

export function roomMessagesPath(roomId: string, limit?: number): string {
  const id = encodeURIComponent(cleanId(roomId))
  const base = `/api/v1/rooms/${id}/messages`
  if (limit === undefined) return base
  return `${base}?limit=${limit}`
}

export function roomGiftsPath(roomId: string): string {
  const id = encodeURIComponent(cleanId(roomId))
  return `/api/v1/rooms/${id}/gifts`
}

// --- Types + parse helpers ---

export type ChatMessage = {
  id: string
  roomId: string
  senderId: string
  senderName: string
  body: string
  createdAt: string
}

export type GiftItem = {
  id: string
  name: string
  price: number
  active: boolean
}

export type GiftOrder = {
  id: string
  totalCoins: number
  replayed: boolean
}

export type WalletBalance = {
  balance: number
}

export function parseChatMessages(json: unknown): ChatMessage[] {
  if (!json || typeof json !== 'object') return []
  const items = (json as Record<string, unknown>).items
  if (!Array.isArray(items)) return []
  return items
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null
      const m = raw as Record<string, unknown>
      return {
        id: typeof m.id === 'string' ? m.id : '',
        roomId: typeof m.room_id === 'string' ? m.room_id : '',
        senderId: typeof m.sender_id === 'string' ? m.sender_id : '',
        senderName: typeof m.sender_name === 'string' ? m.sender_name : '',
        body: typeof m.body === 'string' ? m.body : '',
        createdAt: typeof m.created_at === 'string' ? m.created_at : '',
      } satisfies ChatMessage
    })
    .filter((m): m is ChatMessage => m !== null)
}

export function parseChatMessage(json: unknown): ChatMessage | null {
  if (!json || typeof json !== 'object') return null
  const m = json as Record<string, unknown>
  if (typeof m.id !== 'string') return null
  return {
    id: m.id,
    roomId: typeof m.room_id === 'string' ? m.room_id : '',
    senderId: typeof m.sender_id === 'string' ? m.sender_id : '',
    senderName: typeof m.sender_name === 'string' ? m.sender_name : '',
    body: typeof m.body === 'string' ? m.body : '',
    createdAt: typeof m.created_at === 'string' ? m.created_at : '',
  }
}

export function parseGiftCatalog(json: unknown): GiftItem[] {
  if (!json || typeof json !== 'object') return []
  const items = (json as Record<string, unknown>).items
  if (!Array.isArray(items)) return []
  return items
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null
      const g = raw as Record<string, unknown>
      return {
        id: typeof g.id === 'string' ? g.id : '',
        name: typeof g.name === 'string' ? g.name : '',
        price: typeof g.price === 'number' ? g.price : 0,
        active: typeof g.active === 'boolean' ? g.active : true,
      } satisfies GiftItem
    })
    .filter((g): g is GiftItem => g !== null && g.id.length > 0)
}

export function parseGiftOrder(json: unknown): GiftOrder | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  if (typeof o.id !== 'string') return null
  return {
    id: o.id,
    totalCoins: typeof o.total_coins === 'number' ? o.total_coins : 0,
    replayed: typeof o.replayed === 'boolean' ? o.replayed : false,
  }
}

export function parseWalletBalance(json: unknown): number {
  if (!json || typeof json !== 'object') return 0
  const bal = (json as Record<string, unknown>).balance
  return typeof bal === 'number' ? bal : 0
}

/** JSON body for POST /rooms/{id}/messages. */
export function postMessageBody(body: string): { body: string } {
  return { body }
}

/** JSON body for POST /auth/otp/send. */
export function otpSendBody(email: string): { email: string } {
  return { email }
}

/** JSON body for POST /auth/otp/verify. */
export function otpVerifyBody(email: string, code: string): { email: string; code: string } {
  return { email, code }
}

/** JSON body for POST /wallet/topups. */
export function topupBody(amount: number, reference?: string): { amount: number; reference?: string } {
  if (reference !== undefined) return { amount, reference }
  return { amount }
}

/** JSON body for POST /rooms/{id}/gifts. */
export function sendGiftBody(opts: {
  giftId: string
  receiverId: string
  clientRequestId: string
  count?: number
}): {
  gift_id: string
  receiver_id: string
  count: number
  client_request_id: string
} {
  return {
    gift_id: opts.giftId,
    receiver_id: opts.receiverId,
    count: opts.count ?? 1,
    client_request_id: opts.clientRequestId,
  }
}

/** Authorization header map when token present. */
export function authHeaders(token: string | null | undefined): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }
  if (typeof token === 'string' && token.trim()) {
    headers.Authorization = `Bearer ${token.trim()}`
  }
  return headers
}
