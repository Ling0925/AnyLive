/**
 * Pure helpers for Centrifugo connection + message parsing (no WS in unit tests).
 * Clients still poll HTTP history as a fallback when WS is unavailable.
 */

export type RealtimeToken = {
  token: string
  expiresIn: number
  channels: string[]
}

export function realtimeTokenPath(): string {
  return '/api/v1/realtime/token'
}

export function realtimeTokenBody(roomId: string): { room_id: string } {
  return { room_id: roomId }
}

export function parseRealtimeToken(json: unknown): RealtimeToken | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  if (typeof o.token !== 'string' || !o.token) return null
  const channels = Array.isArray(o.channels)
    ? o.channels.map((c) => String(c))
    : []
  return {
    token: o.token,
    expiresIn: typeof o.expires_in === 'number' ? o.expires_in : 0,
    channels,
  }
}

/** Build WS URL for Centrifugo (optional env VITE_CENTRIFUGO_WS). */
export function centrifugoWsUrl(base?: string | null): string | null {
  const raw = (base ?? '').trim()
  if (!raw) return null
  return raw.replace(/\/$/, '')
}

/**
 * Extract chat body from a Centrifugo publication data blob.
 * Supports both envelope `{ type, payload }` and bare chat message shapes.
 */
export function parseChatPublication(data: unknown): {
  id: string
  body: string
  senderName: string
  senderId: string
} | null {
  if (!data || typeof data !== 'object') return null
  const root = data as Record<string, unknown>
  const payload =
    root.type === 'chat.message' && root.payload && typeof root.payload === 'object'
      ? (root.payload as Record<string, unknown>)
      : root
  const body = typeof payload.body === 'string' ? payload.body : ''
  if (!body) return null
  return {
    id: typeof payload.id === 'string' ? payload.id : `ws-${Date.now()}`,
    body,
    senderName:
      typeof payload.sender_name === 'string'
        ? payload.sender_name
        : typeof payload.senderName === 'string'
          ? payload.senderName
          : '',
    senderId:
      typeof payload.sender_id === 'string'
        ? payload.sender_id
        : typeof payload.senderId === 'string'
          ? payload.senderId
          : '',
  }
}

export type CentrifugoChatHandlers = {
  onMessage: (msg: {
    id: string
    body: string
    senderName: string
    senderId: string
  }) => void
  onStatus?: (status: 'connecting' | 'open' | 'closed' | 'error') => void
}

/**
 * Minimal Centrifugo JSON protocol client (connect + subscribe).
 * Returns a stop function. Falls back silently if WS cannot open.
 */
export function connectCentrifugoChat(opts: {
  wsUrl: string
  token: string
  channel: string
  handlers: CentrifugoChatHandlers
}): () => void {
  let closed = false
  let ws: WebSocket | null = null
  let cmdId = 0
  const nextId = () => {
    cmdId += 1
    return cmdId
  }

  try {
    opts.handlers.onStatus?.('connecting')
    ws = new WebSocket(opts.wsUrl)
  } catch {
    opts.handlers.onStatus?.('error')
    return () => {
      closed = true
    }
  }

  ws.onopen = () => {
    if (closed || !ws) return
    opts.handlers.onStatus?.('open')
    ws.send(
      JSON.stringify({
        id: nextId(),
        connect: { token: opts.token },
      }),
    )
    ws.send(
      JSON.stringify({
        id: nextId(),
        subscribe: { channel: opts.channel },
      }),
    )
  }

  ws.onmessage = (ev) => {
    if (closed) return
    let data: unknown
    try {
      data = JSON.parse(String(ev.data))
    } catch {
      return
    }
    if (!data || typeof data !== 'object') return
    const root = data as Record<string, unknown>
    const push = root.push as Record<string, unknown> | undefined
    if (push && typeof push === 'object') {
      const pub = push.pub as Record<string, unknown> | undefined
      if (pub && typeof pub === 'object' && 'data' in pub) {
        const msg = parseChatPublication(pub.data)
        if (msg) opts.handlers.onMessage(msg)
      }
    }
    const result = root.result as Record<string, unknown> | undefined
    if (result && Array.isArray(result.publications)) {
      for (const pub of result.publications) {
        if (pub && typeof pub === 'object' && 'data' in (pub as object)) {
          const msg = parseChatPublication((pub as { data: unknown }).data)
          if (msg) opts.handlers.onMessage(msg)
        }
      }
    }
  }

  ws.onerror = () => {
    opts.handlers.onStatus?.('error')
  }

  ws.onclose = () => {
    opts.handlers.onStatus?.('closed')
  }

  return () => {
    closed = true
    try {
      ws?.close()
    } catch {
      // ignore
    }
    ws = null
  }
}
