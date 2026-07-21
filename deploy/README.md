# Deploy (local / test)

## Dependencies only

```bash
docker compose -f deploy/docker-compose.yml up -d
```

Postgres, Redis, NATS, MinIO, Centrifugo, SRS.

## Full test stack (API + Admin)

```bash
./scripts/deploy-test.sh
```

Or:

```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test --profile app up -d --build
```

| Service | URL |
|---|---|
| API | http://localhost:8088/health |
| Admin | http://localhost:8090/ |
| SRS HLS | http://localhost:8080/live |
| Centrifugo | http://localhost:8001 |

- Dev OTP code: **123456** (`APP_ENV=local`)
- API uses Postgres (`USE_POSTGRES=1`) and auto-migrates on boot
- Admin is built with `VITE_API_BASE=http://localhost:8088` (browser → host port)

### Stop

```bash
docker compose -f deploy/docker-compose.yml --profile app down
# keep volumes:
docker compose -f deploy/docker-compose.yml --profile app down
# wipe DB:
docker compose -f deploy/docker-compose.yml --profile app down -v
```

### Rebuild after code changes

```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test --profile app up -d --build api admin
```
