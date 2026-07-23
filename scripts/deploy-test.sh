#!/usr/bin/env bash
# 构建并启动 AnyLive 测试栈：依赖 + API + admin-web。
# Control-plane stack bring-up only — does NOT close V-BE-1/2 or sign risk-accept.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test)
REPORTS_DIR="${DOGFOOD_REPORT_DIR:-$ROOT/reports}"
SMOKE_FAILED=0

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
echo "FEATURE_PK=0 FEATURE_COHOST=0（P1-safe；见 deploy/.env.test）"
echo
echo "OBS 开播（主播调用 media/publish 之后）："
echo "  服务:       自定义…"
echo "  服务器:     从 push_url 推导（本地常为 rtmp://localhost:1935/live）"
echo "  串流密钥:   POST /api/v1/rooms/{id}/media/publish 返回的签名 token"
echo "              （格式 roomId?exp=&sig= — 不是裸房间 UUID；必须整段粘贴）"
echo "  观看 HLS:   GET /api/v1/rooms/{id}/media/play  或 H5 ?room={id}"
echo "  完整手册:   docs/runbooks/go-live-local.md · scripts/dogfood-media.md"
echo
echo "停止: docker compose -f deploy/docker-compose.yml --profile app down"
echo
echo "======== 一键重跑冒烟（栈保持运行）========"
echo "  mkdir -p reports"
echo "  DOGFOOD_REPORT_DIR=reports API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 \\"
echo "    ./scripts/dogfood-api-smoke.sh"
echo "  # OBS 周：留下 live 房间，便于粘贴推流信息"
echo "  DOGFOOD_REPORT_DIR=reports SKIP_FORCE_CLOSE=1 API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 \\"
echo "    ./scripts/dogfood-10min-path.sh"
echo "  API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 SRS_API_BASE=http://127.0.0.1:1985 \\"
echo "    ./scripts/dogfood-media-smoke.sh"
echo "  # Stage 再冒烟（真 OTP + 跳过 mock）："
echo "  # DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=https://api.stage.example ./scripts/dogfood-api-smoke.sh"
echo "=========================================="
echo
echo "NOTE: deploy-test + dogfood PASS 仅为控制面就绪，不关闭 V-BE-1/2，也不等于风险接受书已签。"
echo

# 可选：对刚拉起的栈跑 dogfood 冒烟。
# SKIP_DOGFOOD_SMOKE=1 跳过。
# 默认：失败不退出，栈保持运行（方便 OBS 周调试）。
# DOGFOOD_SMOKE_REQUIRED=1：冒烟失败则整体 exit non-zero（CI / 严格本地）。
if [ "${SKIP_DOGFOOD_SMOKE:-0}" = "1" ]; then
  echo "==> SKIP_DOGFOOD_SMOKE=1 — 跳过 dogfood 冒烟"
else
  mkdir -p "$REPORTS_DIR"
  UTC_STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

  echo "==> dogfood API 冒烟 (log → ${REPORTS_DIR}/dogfood-api-smoke-${UTC_STAMP}.log)"
  if DOGFOOD_REPORT_DIR="$REPORTS_DIR" \
      API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 \
      "$ROOT/scripts/dogfood-api-smoke.sh"; then
    echo "dogfood-api-smoke: 通过"
  else
    echo "WARN: dogfood-api-smoke 失败（栈仍在运行；可重跑: ./scripts/dogfood-api-smoke.sh）" >&2
    SMOKE_FAILED=1
  fi

  echo
  echo "==> dogfood 媒体冒烟 (log → ${REPORTS_DIR}/dogfood-media-smoke-${UTC_STAMP}.log)"
  # media smoke has no built-in tee; capture here
  if API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 SRS_API_BASE=http://127.0.0.1:1985 \
      "$ROOT/scripts/dogfood-media-smoke.sh" 2>&1 | tee "${REPORTS_DIR}/dogfood-media-smoke-${UTC_STAMP}.log"; then
    echo "dogfood-media-smoke: 通过"
  else
    echo "WARN: dogfood-media-smoke 失败（栈仍在运行；可重跑: ./scripts/dogfood-media-smoke.sh）" >&2
    SMOKE_FAILED=1
  fi

  if [ "$SMOKE_FAILED" -ne 0 ]; then
    if [ "${DOGFOOD_SMOKE_REQUIRED:-0}" = "1" ] || [ "${DOGFOOD_SMOKE_REQUIRED:-0}" = "true" ]; then
      echo "ERROR: dogfood smoke failed and DOGFOOD_SMOKE_REQUIRED=1 — exiting non-zero (stack still up)" >&2
      exit 1
    fi
    echo "NOTE: smoke failure non-fatal for stack bring-up (set DOGFOOD_SMOKE_REQUIRED=1 to fail hard)."
  fi
fi

echo
echo "======== Human OBS week checklist ========"
echo "  [ ] OBS Custom: Server from push_url + full Stream Key (?exp=&sig=)"
echo "  [ ] Start Streaming → H5/Flutter play HLS"
echo "  [ ] Stop / unpublish → room leaves live"
echo "  Tip: SKIP_FORCE_CLOSE=1 ./scripts/dogfood-10min-path.sh  # leave room live"
echo "  Does NOT close V-BE-1/2 or plan 06 #1/#2 without human sign-off."
echo "=========================================="
