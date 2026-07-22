import { describe, expect, it } from 'vitest'
import {
  apiUrl,
  authHeaders,
  giftsPath,
  normalizeApiBase,
  otpSendBody,
  otpSendPath,
  otpVerifyBody,
  otpVerifyPath,
  parseChatMessage,
  parseChatMessages,
  parseGiftCatalog,
  parseGiftOrder,
  parseWalletBalance,
  postMessageBody,
  roomGiftsPath,
  roomMessagesPath,
  sendGiftBody,
  topupBody,
  walletPath,
  walletTopupPath,
  payProductsPath,
  payOrdersPath,
  paySandboxCompletePath,
  parsePayProducts,
  parsePayOrder,
  createPayOrderBody,
} from './chatApi'

describe('url helpers', () => {
  it('normalizes trailing slash', () => {
    expect(normalizeApiBase('http://localhost:8088/')).toBe('http://localhost:8088')
    expect(normalizeApiBase('http://localhost:8088')).toBe('http://localhost:8088')
  })

  it('joins base and path', () => {
    expect(apiUrl('http://localhost:8088/', '/api/v1/gifts')).toBe(
      'http://localhost:8088/api/v1/gifts',
    )
    expect(apiUrl('http://localhost:8088', 'api/v1/gifts')).toBe(
      'http://localhost:8088/api/v1/gifts',
    )
  })
})

describe('path builders', () => {
  it('otp paths', () => {
    expect(otpSendPath()).toBe('/api/v1/auth/otp/send')
    expect(otpVerifyPath()).toBe('/api/v1/auth/otp/verify')
  })

  it('wallet / gifts paths', () => {
    expect(giftsPath()).toBe('/api/v1/gifts')
    expect(walletPath()).toBe('/api/v1/wallet')
    expect(walletTopupPath()).toBe('/api/v1/wallet/topups')
  })

  it('pay paths', () => {
    expect(payProductsPath()).toBe('/api/v1/pay/products')
    expect(payOrdersPath()).toBe('/api/v1/pay/orders')
    expect(paySandboxCompletePath('ord-1')).toBe(
      '/api/v1/pay/orders/ord-1/sandbox-complete',
    )
  })

  it('room messages path with optional limit', () => {
    expect(roomMessagesPath('r1')).toBe('/api/v1/rooms/r1/messages')
    expect(roomMessagesPath('r1', 20)).toBe('/api/v1/rooms/r1/messages?limit=20')
    expect(roomMessagesPath('/uuid-x/')).toBe('/api/v1/rooms/uuid-x/messages')
  })

  it('room gifts path', () => {
    expect(roomGiftsPath('abc')).toBe('/api/v1/rooms/abc/gifts')
    expect(roomGiftsPath('/abc/')).toBe('/api/v1/rooms/abc/gifts')
  })
})

describe('request bodies', () => {
  it('builds otp bodies', () => {
    expect(otpSendBody('a@b.com')).toEqual({ email: 'a@b.com' })
    expect(otpVerifyBody('a@b.com', '123456')).toEqual({
      email: 'a@b.com',
      code: '123456',
    })
  })

  it('builds message / topup / gift bodies', () => {
    expect(postMessageBody('hi')).toEqual({ body: 'hi' })
    expect(topupBody(100)).toEqual({ amount: 100 })
    expect(topupBody(50, 'ref-1')).toEqual({ amount: 50, reference: 'ref-1' })
    expect(
      sendGiftBody({
        giftId: 'g1',
        receiverId: 'u2',
        clientRequestId: 'c1',
      }),
    ).toEqual({
      gift_id: 'g1',
      receiver_id: 'u2',
      count: 1,
      client_request_id: 'c1',
    })
  })
})

describe('authHeaders', () => {
  it('always sets content-type; adds bearer when token present', () => {
    expect(authHeaders(null)).toEqual({ 'Content-Type': 'application/json' })
    expect(authHeaders('tok')).toEqual({
      'Content-Type': 'application/json',
      Authorization: 'Bearer tok',
    })
  })
})

describe('parse helpers', () => {
  it('parses chat list and single message', () => {
    const list = parseChatMessages({
      items: [
        {
          id: 'm1',
          room_id: 'r',
          sender_id: 'u',
          sender_name: 'Ann',
          body: 'hello',
          created_at: 't',
        },
      ],
    })
    expect(list).toHaveLength(1)
    expect(list[0].body).toBe('hello')
    expect(parseChatMessages({})).toEqual([])
    expect(parseChatMessages(null)).toEqual([])

    const one = parseChatMessage({
      id: 'm2',
      room_id: 'r',
      sender_id: 'u',
      sender_name: 'Bob',
      body: 'yo',
      created_at: 't2',
    })
    expect(one?.senderName).toBe('Bob')
    expect(parseChatMessage({})).toBeNull()
  })

  it('parses gift catalog and order', () => {
    const gifts = parseGiftCatalog({
      items: [
        { id: 'g1', name: 'Rose', price: 10, active: true },
        { id: '', name: 'skip', price: 1 },
      ],
    })
    expect(gifts).toEqual([{ id: 'g1', name: 'Rose', price: 10, active: true }])
    expect(parseGiftCatalog(null)).toEqual([])

    const order = parseGiftOrder({ id: 'o1', total_coins: 20, replayed: false })
    expect(order).toEqual({ id: 'o1', totalCoins: 20, replayed: false })
    expect(parseGiftOrder({})).toBeNull()
  })

  it('parses wallet balance', () => {
    expect(parseWalletBalance({ balance: 42 })).toBe(42)
    expect(parseWalletBalance({})).toBe(0)
    expect(parseWalletBalance(null)).toBe(0)
  })
})

describe('pay parse / body', () => {
  it('parses products and orders', () => {
    expect(
      parsePayProducts({
        items: [{ id: 'p1', sku: 'coins_100', title: '100', coins: 100, amount: '6.00', currency: 'CNY' }],
      }),
    ).toEqual([
      { id: 'p1', sku: 'coins_100', title: '100', coins: 100, amount: '6.00', currency: 'CNY' },
    ])
    expect(
      parsePayOrder({
        id: 'o1',
        status: 'credited',
        coins: 100,
        amount: '6.00',
        currency: 'CNY',
        channel: 'mock',
      }),
    ).toEqual({
      id: 'o1',
      status: 'credited',
      coins: 100,
      amount: '6.00',
      currency: 'CNY',
      channel: 'mock',
    })
  })

  it('builds create order body', () => {
    expect(createPayOrderBody({ productId: 'p1' })).toEqual({
      product_id: 'p1',
      channel: 'mock',
    })
    expect(createPayOrderBody({ productId: 'p1', clientRequestId: 'c1', channel: 'mock' })).toEqual({
      product_id: 'p1',
      channel: 'mock',
      client_request_id: 'c1',
    })
  })
})
