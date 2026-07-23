import { describe, expect, it } from 'vitest'
import {
  centrifugoWsUrl,
  parseChatPublication,
  parseRealtimeToken,
  realtimeTokenBody,
  realtimeTokenPath,
} from './realtime'

describe('realtime helpers', () => {
  it('paths and body', () => {
    expect(realtimeTokenPath()).toBe('/api/v1/realtime/token')
    expect(realtimeTokenBody('r1')).toEqual({ room_id: 'r1' })
  })

  it('parseRealtimeToken', () => {
    expect(parseRealtimeToken(null)).toBeNull()
    expect(
      parseRealtimeToken({
        token: 't',
        expires_in: 60,
        channels: ['room:1'],
      }),
    ).toEqual({ token: 't', expiresIn: 60, channels: ['room:1'] })
  })

  it('centrifugoWsUrl trims trailing slash', () => {
    expect(centrifugoWsUrl(null)).toBeNull()
    expect(centrifugoWsUrl('')).toBeNull()
    expect(centrifugoWsUrl('ws://localhost:8000/connection/websocket/')).toBe(
      'ws://localhost:8000/connection/websocket',
    )
  })

  it('parseChatPublication envelope', () => {
    const msg = parseChatPublication({
      type: 'chat.message',
      payload: {
        id: 'm1',
        body: 'hi',
        sender_id: 'u1',
        sender_name: 'Alice',
      },
    })
    expect(msg).toEqual({
      id: 'm1',
      body: 'hi',
      senderName: 'Alice',
      senderId: 'u1',
    })
  })
})
