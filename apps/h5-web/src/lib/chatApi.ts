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

export function mePath(): string {
  return '/api/v1/me'
}

export function meExportPath(): string {
  return '/api/v1/me/export'
}

/** JSON body for PATCH /me (age / privacy declarations). */
export function patchMeBody(opts: {
  ageConfirmed?: boolean
  privacyAccepted?: boolean
  displayName?: string
}): {
  age_confirmed?: boolean
  privacy_accepted?: boolean
  display_name?: string
} {
  const body: {
    age_confirmed?: boolean
    privacy_accepted?: boolean
    display_name?: string
  } = {}
  if (opts.ageConfirmed !== undefined) body.age_confirmed = opts.ageConfirmed
  if (opts.privacyAccepted !== undefined) body.privacy_accepted = opts.privacyAccepted
  if (opts.displayName !== undefined) body.display_name = opts.displayName
  return body
}

export function payProductsPath(): string {
  return '/api/v1/pay/products'
}

export function payChannelsPath(): string {
  return '/api/v1/pay/channels'
}

export function payOrdersPath(): string {
  return '/api/v1/pay/orders'
}

export function payOrderPath(orderId: string): string {
  const id = encodeURIComponent(cleanId(orderId))
  return `/api/v1/pay/orders/${id}`
}

export function paySandboxCompletePath(orderId: string): string {
  const id = encodeURIComponent(cleanId(orderId))
  return `/api/v1/pay/orders/${id}/sandbox-complete`
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

export type PayProduct = {
  id: string
  sku: string
  title: string
  coins: number
  amount: string
  currency: string
}

export type PayOrder = {
  id: string
  status: string
  coins: number
  amount: string
  currency: string
  channel: string
}

export function parsePayProducts(json: unknown): PayProduct[] {
  if (!json || typeof json !== 'object') return []
  const items = (json as Record<string, unknown>).items
  if (!Array.isArray(items)) return []
  return items
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null
      const p = raw as Record<string, unknown>
      return {
        id: typeof p.id === 'string' ? p.id : '',
        sku: typeof p.sku === 'string' ? p.sku : '',
        title: typeof p.title === 'string' ? p.title : '',
        coins: typeof p.coins === 'number' ? p.coins : 0,
        amount: typeof p.amount === 'string' ? p.amount : '',
        currency: typeof p.currency === 'string' ? p.currency : '',
      } satisfies PayProduct
    })
    .filter((p): p is PayProduct => p !== null && p.id.length > 0)
}

export function parsePayOrder(json: unknown): PayOrder | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  if (typeof o.id !== 'string') return null
  return {
    id: o.id,
    status: typeof o.status === 'string' ? o.status : '',
    coins: typeof o.coins === 'number' ? o.coins : 0,
    amount: typeof o.amount === 'string' ? o.amount : '',
    currency: typeof o.currency === 'string' ? o.currency : '',
    channel: typeof o.channel === 'string' ? o.channel : '',
  }
}

/** JSON body for POST /pay/orders. */
export function createPayOrderBody(opts: {
  productId: string
  channel?: string
  clientRequestId?: string
}): {
  product_id: string
  channel: string
  client_request_id?: string
} {
  const body: {
    product_id: string
    channel: string
    client_request_id?: string
  } = {
    product_id: opts.productId,
    channel: opts.channel ?? 'mock',
  }
  if (opts.clientRequestId) body.client_request_id = opts.clientRequestId
  return body
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

// --- Creator center / analytics / interactive (P3–P4) ---

export function creatorStatsPath(): string {
  return '/api/v1/me/creator'
}

export function eventsPath(): string {
  return '/api/v1/events'
}

export function roomInteractivePath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/interactive`
}

export function roomInteractiveInvitePath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/interactive/invite`
}

export function roomInteractiveRespondPath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/interactive/respond`
}

export function roomInteractiveLeavePath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/interactive/leave`
}

export function roomPkPath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/pk`
}

export function roomPkStartPath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/pk/start`
}

export function roomPkEndPath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/pk/end`
}

export function roomLivekitJoinPath(roomId: string): string {
  return `/api/v1/rooms/${cleanId(roomId)}/livekit/join`
}

export type CreatorStats = {
  followerCount: number
  followingCount: number
  liveRooms: number
  totalRooms: number
  giftCoinsReceived: number
  giftCreditEntries: number
}

export function parseCreatorStats(json: unknown): CreatorStats | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  const num = (v: unknown) => (typeof v === 'number' ? v : 0)
  return {
    followerCount: num(o.follower_count),
    followingCount: num(o.following_count),
    liveRooms: num(o.live_rooms),
    totalRooms: num(o.total_rooms),
    giftCoinsReceived: num(o.gift_coins_received),
    giftCreditEntries: num(o.gift_credit_entries),
  }
}

export type PkSession = {
  id: string
  roomAId: string
  roomBId: string
  status: string
  scoreA: number
  scoreB: number
  winnerRoomId: string | null
}

export function parsePkSession(json: unknown): PkSession | null {
  if (!json || typeof json !== 'object') return null
  const root = json as Record<string, unknown>
  const o = (root.session && typeof root.session === 'object'
    ? root.session
    : root) as Record<string, unknown>
  if (typeof o.id !== 'string') return null
  const num = (v: unknown) => (typeof v === 'number' ? v : 0)
  return {
    id: o.id,
    roomAId: typeof o.room_a_id === 'string' ? o.room_a_id : '',
    roomBId: typeof o.room_b_id === 'string' ? o.room_b_id : '',
    status: typeof o.status === 'string' ? o.status : '',
    scoreA: num(o.score_a),
    scoreB: num(o.score_b),
    winnerRoomId: typeof o.winner_room_id === 'string' ? o.winner_room_id : null,
  }
}

/** JSON body for POST /events batch. */
export function clientEventsBody(
  events: Array<{
    name: string
    props?: Record<string, unknown>
    clientEventId?: string
  }>,
): { events: Array<Record<string, unknown>> } {
  return {
    events: events.map((e) => {
      const item: Record<string, unknown> = { name: e.name }
      if (e.props) item.props = e.props
      if (e.clientEventId) item.client_event_id = e.clientEventId
      return item
    }),
  }
}

export function interactiveInviteBody(inviteeId: string): { invitee_id: string } {
  return { invitee_id: inviteeId }
}

export function startPkBody(opts: {
  opponentRoomId: string
  durationSecs?: number
}): { opponent_room_id: string; duration_secs?: number } {
  const body: { opponent_room_id: string; duration_secs?: number } = {
    opponent_room_id: opts.opponentRoomId,
  }
  if (opts.durationSecs != null) body.duration_secs = opts.durationSecs
  return body
}

// --- Search (WBS E6.3) ---

export function searchPath(
  q: string,
  opts?: { type?: 'all' | 'users' | 'rooms'; limit?: number },
): string {
  const params = new URLSearchParams()
  params.set('q', q)
  if (opts?.type) params.set('type', opts.type)
  if (opts?.limit != null) params.set('limit', String(opts.limit))
  return `/api/v1/search?${params.toString()}`
}

export type SearchUserHit = {
  id: string
  displayName: string
}

export type SearchRoomHit = {
  id: string
  title: string
  status: string
  ownerId: string
}

export type SearchResult = {
  users: SearchUserHit[]
  rooms: SearchRoomHit[]
}

export function parseSearchResult(json: unknown): SearchResult {
  if (!json || typeof json !== 'object') return { users: [], rooms: [] }
  const root = json as Record<string, unknown>
  const usersRaw = Array.isArray(root.users) ? root.users : []
  const roomsRaw = Array.isArray(root.rooms) ? root.rooms : []
  const users = usersRaw
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null
      const u = raw as Record<string, unknown>
      const id = typeof u.id === 'string' ? u.id : ''
      if (!id) return null
      return {
        id,
        displayName: typeof u.display_name === 'string' ? u.display_name : '',
      } satisfies SearchUserHit
    })
    .filter((u): u is SearchUserHit => u !== null)
  const rooms = roomsRaw
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null
      const r = raw as Record<string, unknown>
      const id = typeof r.id === 'string' ? r.id : ''
      if (!id) return null
      return {
        id,
        title: typeof r.title === 'string' ? r.title : '',
        status: typeof r.status === 'string' ? r.status : '',
        ownerId: typeof r.owner_id === 'string' ? r.owner_id : '',
      } satisfies SearchRoomHit
    })
    .filter((r): r is SearchRoomHit => r !== null)
  return { users, rooms }
}

// --- Hot feed (public discover) ---

export function feedHotPath(limit = 12): string {
  const n = Math.max(1, Math.min(limit, 50))
  return `/api/v1/feed/hot?limit=${n}`
}

/** Room card for hot/discover lists (subset of API room fields). */
export type FeedRoom = {
  id: string
  title: string
  status: string
  ownerId: string
}

/** Parse `/api/v1/feed/hot` body: `{ items: Room[] }` or bare array. */
export function parseFeedRooms(json: unknown): FeedRoom[] {
  let raw: unknown[] = []
  if (Array.isArray(json)) {
    raw = json
  } else if (json && typeof json === 'object') {
    const root = json as Record<string, unknown>
    if (Array.isArray(root.items)) raw = root.items
    else if (Array.isArray(root.rooms)) raw = root.rooms
  }
  return raw
    .map((item) => {
      if (!item || typeof item !== 'object') return null
      const r = item as Record<string, unknown>
      const id = typeof r.id === 'string' ? r.id : ''
      if (!id) return null
      return {
        id,
        title: typeof r.title === 'string' ? r.title : '',
        status: typeof r.status === 'string' ? r.status : '',
        ownerId: typeof r.owner_id === 'string' ? r.owner_id : '',
      } satisfies FeedRoom
    })
    .filter((r): r is FeedRoom => r !== null)
}
