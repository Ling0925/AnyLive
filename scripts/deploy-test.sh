#!/usr/bin/env bash
# 构建并启动 AnyLive 测试栈：依赖 + API + admin-web。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test)

echo "==> docker compose config"
"${COMPOSE[@]}" config --quiet

echo "==> 构建 api + admin（首次可能需数分钟）"
"${COMPOSE[@]}" --profile app build api admin

echo "==> 启动完整测试 profile"
"${COMPOSE[@]}" --profile app up -d

echo "==> 等待 API 健康"
for i in $(seq 1 60); do
  if curl -fsS http://127.0.0.1:8088/health >/dev/null 2>&1; then
    echo "API 健康"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "API 健康检查超时" >&2
    "${COMPOSE[@]}" --profile app logs --tail=80 api || true
    exit 1
  fi
  sleep 2
done

echo "==> 等待管理后台"
for i in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8090/ >/dev/null 2>&1; then
    echo "管理后台可访问"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "管理后台超时" >&2
    "${COMPOSE[@]}" --profile app logs --tail=40 admin || true
    exit 1
  fi
  sleep 1
done

echo
echo "测试栈已就绪："
echo "  API:        http://localhost:8088/health"
echo "  管理后台:   http://localhost:8090/"
echo "  SRS HLS:    http://localhost:8080/live"
echo "  Centrifugo: http://localhost:8001"
echo
echo "开发 OTP 验证码: 123456  （ALLOW_DEV_OTP=1 / APP_ENV=local）"
echo
echo "OBS 开播（主播调用 media/publish 之后）："
echo "  服务:       自定义…"
echo "  服务器:     rtmp://localhost:1935/live"
echo "  串流密钥:   POST /api/v1/rooms/{id}/media/publish 返回的签名 token"
echo "              （格式 roomId?exp=&sig= — 不是裸房间 UUID）"
echo "  观看 HLS:   GET /api/v1/rooms/{id}/media/play  或 H5 ?room={id}"
echo "  完整手册:   docs/runbooks/go-live-local.md"
echo
echo "停止: docker compose -f deploy/docker-compose.yml --profile app down"
echo

# 可选：对刚拉起的栈跑 dogfood 冒烟。
# SKIP_DOGFOOD_SMOKE=1 跳过；失败不退出，栈保持运行。
if [ "${SKIP_DOGFOOD_SMOKE:-0}" = "1" ]; then
  echo "==> SKIP_DOGFOOD_SMOKE=1 — 跳过 dogfood 冒烟"
else
  echo "==> dogfood API 冒烟"
  if API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 \
      "$ROOT/scripts/dogfood-api-smoke.sh"; then
    echo "dogfood-api-smoke: 通过"
  else
    echo "WARN: dogfood-api-smoke 失败（栈仍在运行；可重跑: ./scripts/dogfood-api-smoke.sh）" >&2
  fi

  echo
  echo "==> dogfood 媒体冒烟"
  if API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 SRS_API_BASE=http://127.0.0.1:1985 \
      "$ROOT/scripts/dogfood-media-smoke.sh"; then
    echo "dogfood-media-smoke: 通过"
  else
    echo "WARN: dogfood-media-smoke 失败（栈仍在运行；可重跑: ./scripts/dogfood-media-smoke.sh）" >&2
  fi
fi
