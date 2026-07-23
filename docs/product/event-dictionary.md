# Client event dictionary (P4)

Canonical names for `POST /api/v1/events`. Clients should batch ≤50 events and prefer stable `client_event_id` for retries.

| name | when | recommended props |
|---|---|---|
| `room.view` | Enter / load room detail | `room_id`, `status` |
| `gift.tap` | User taps send gift (after success) | `room_id`, `gift_id` |
| `chat.send` | Chat message accepted | `room_id` |
| `auth.login` | OTP verify success | `method=otp` |
| `pay.order_create` | Pay order created | `product_id`, `channel` |
| `pay.order_credit` | Sandbox/webhook credited | `order_id`, `channel` |
| `pk.start` | Host starts PK | `room_id`, `opponent_room_id` |
| `pk.end` | PK ends | `room_id`, `winner_room_id?` |
| `cohost.invite` | Host invites co-host | `room_id`, `invitee_id` |
| `feed.impression` | Feed card visible | `room_id`, `feed=hot\|following` |

## Wire status

| client | wired |
|---|---|
| Flutter room page | `room.view`, `gift.tap`, `chat.send`, `pk.start`, `pk.end`, `cohost.invite` |
| Flutter login | `auth.login` |
| Flutter wallet | `pay.order_create`, `pay.order_credit` |
| Flutter feed | `feed.impression` (hot / following, top 20) |
| H5 watch | `room.view`, `gift.tap`, `chat.send`, `auth.login`, `pay.order_create`, `pay.order_credit` (when authed) |
| Admin | metrics scrape UI only (no product events) |

Additional names may be added without OpenAPI change (`name` is free string). Ingest is feature-gated by `FEATURE_CLIENT_EVENTS`.
