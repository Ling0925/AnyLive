# Wave2 人工自测打包清单

> 把本地测试栈 + 三端客户端 + 控制面 dogfood 收成**一套可复用的人肉自测包**。  
> 本清单**不**关闭 V-BE-1/2、V-FL-1/2、V-AD-1、V-ALL-1；控制面 PASS ≠ 出口签字。  
> 更完整的开播步骤见 [go-live-local.md](./go-live-local.md)；队列与 V-FL-2 录屏见 [dogfood-cohort.md](./dogfood-cohort.md)。

## 0. 目标产物

| 产物 | 用途 |
|---|---|
| 运行中的 test 栈（API `:8088`、Admin Docker `:8090`、SRS HLS `:8080`） | 真人 OBS / 客户端自测 |
| Flutter APK（debug 或 release） | 模拟器 / 真机装机 |
| admin-web / h5-web 静态 build + preview | 浏览器运营与观看 |
| dogfood 日志（`reports/dogfood-*.log`） | 控制面就绪证据（非签字） |

## 1. 重部署测试栈

从仓库根目录：

```bash
./scripts/deploy-test.sh
```

- 构建并拉起 Postgres / Redis / NATS / MinIO / Centrifugo / SRS / API / admin 镜像。
- 默认在栈就绪后跑 `dogfood-api-smoke` + `dogfood-media-smoke`（日志 tee → `reports/`）。
- 冒烟失败默认**不**拆栈（便于 OBS 周调试）；严格失败：`DOGFOOD_SMOKE_REQUIRED=1 ./scripts/deploy-test.sh`。

仅要栈、稍后自行冒烟：

```bash
SKIP_DOGFOOD_SMOKE=1 ./scripts/deploy-test.sh
```

停止：

```bash
docker compose -f deploy/docker-compose.yml --profile app down
```

## 2. 构建 Flutter APK

```bash
# Android 模拟器（默认 API → 10.0.2.2 回环到宿主机 8088）
./scripts/build-mobile-apk.sh debug
# 或
./scripts/build-mobile-apk.sh release

# 真机 / 同局域网：把 API 指到宿主机 LAN IP
API_BASE_URL=http://192.168.x.x:8088 ./scripts/build-mobile-apk.sh debug
```

| 目标 | `API_BASE_URL` 建议 |
|---|---|
| Android 模拟器 | `http://10.0.2.2:8088`（脚本默认） |
| iOS 模拟器 / 桌面 | `http://localhost:8088` 或本机可达地址 |
| 真机 | 宿主机局域网 IP，如 `http://192.168.x.x:8088` |

输出：Flutter 默认路径 + 拷贝至 `reports/apk/anylive-local-<mode>-<stamp>.apk`。  
安装示例：`adb install -r reports/apk/...apk`（真机也可 `adb reverse tcp:8088 tcp:8088`）。

## 3. 构建 admin-web 与 h5-web

两包均需 `pnpm`；`VITE_API_BASE` **构建时注入**。

```bash
# 管理后台（源码 build + preview；Docker admin 仍是 :8090）
cd apps/admin-web
VITE_API_BASE=http://localhost:8088 pnpm build
pnpm preview
# Vite 默认 preview 常为 http://localhost:4173/（若端口占用会顺延）

# H5 观看页
cd apps/h5-web
VITE_API_BASE=http://localhost:8088 pnpm build
pnpm preview
# 默认 preview: http://localhost:4173/ — 与 admin 同时 preview 时请显式错开端口，例如：
#   pnpm preview -- --port 4174
# 打开: http://localhost:4174/?room=<room-uuid>
```

| 端 | 热更新 dev | 静态 preview | Docker 测试栈 |
|---|---|---|---|
| admin-web | `pnpm dev`（Vite 默认端口） | `pnpm build` + `pnpm preview` | `http://localhost:8090/` |
| h5-web | `pnpm dev` → 常为 `:5173` | `pnpm build` + `pnpm preview`（常为 `:4173`） | — |

真机浏览器访问时，`VITE_API_BASE` 须为手机可达的 API 地址（LAN IP，勿写仅宿主机 `localhost`）。

## 4. Dogfood 脚本顺序（控制面）

栈健康后，**按此顺序**跑（与 [dogfood-cohort.md](./dogfood-cohort.md) 预检一致）：

```bash
# 1) 礼物目录（Rose/Heart/Rocket upsert）
./scripts/dogfood-gift-seed.sh

# 2) 主播+观众 10 分钟控制面路径（OBS 周建议留下 live 房）
SKIP_FORCE_CLOSE=1 DOGFOOD_REPORT_DIR=reports ./scripts/dogfood-10min-path.sh
# 期望末行含 DOGFOOD_10MIN_PATH_PASS

# 3) 全量 API 冒烟（mute/ban/pay/export 等）
DOGFOOD_REPORT_DIR=reports ./scripts/dogfood-api-smoke.sh
# 期望末行含 DOGFOOD_API_SMOKE_PASS
```

共用 env：`API_BASE`（默认 `http://localhost:8088`）、`OTP_CODE`（测试栈 `123456`）、`DOGFOOD_ADMIN_EMAIL`。  
可选媒体面：`./scripts/dogfood-media-smoke.sh`。  
**控制面绿只证明 API 就绪，不替代 OBS 推流、真机路径或任何风险接受签字。**

## 5. 仍须人工签字 / 操作的项

脚本与 CI **不得**自动标 done：

| 项 | 说明 | 入口 |
|---|---|---|
| **Risk-accept（OTP / WS soak）** | 草案未签 ≠ V-BE-1 / V-BE-2 关闭 | [otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md) · [ws-1k-soak-risk-accept.md](./ws-1k-soak-risk-accept.md) |
| **设备矩阵（V-FL-1）** | Mid Android + 近两代 iPhone + H5；Pass **仅人工勾选** | `./scripts/device-matrix-prefill.sh` → `reports/device-matrix-*.md` |
| **OBS 周** | 真人连续推流 ≥7 天（可轮值）；Server from `push_url` + 完整 `stream_key`（含 `?exp=&sig=`） | [go-live-local.md](./go-live-local.md) §4 · [dogfood-cohort.md](./dogfood-cohort.md) |
| **缺陷会（V-ALL-1）** | 无开放 P0 的纪要 | [p1-parallel-tracks.md](../product/p1-parallel-tracks.md) V-ALL-1 |
| **V-FL-2 录屏** | 真人 login→feed→HLS→chat→gift→end-state + **Recording URL** | [dogfood-cohort.md](./dogfood-cohort.md) §V-FL-2 |
| **V-AD-1 运营演示** | 15 min 后台走查 + 签字 | [admin-ops-15min-demo.md](./admin-ops-15min-demo.md) |

## 6. P1-safe：FEATURE_PK / FEATURE_COHOST 必须为 false

测试栈 `deploy/.env.test` 默认 `FEATURE_PK=0` / `FEATURE_COHOST=0`。自测打包前确认：

```bash
curl -fsS http://127.0.0.1:8088/api/v1/meta | python3 -m json.tool
# 期望: features.pk == false 且 features.cohost == false
```

- `dogfood-api-smoke` / `dogfood-10min-path` 在开关误开时默认 **FAIL**（仅 `ALLOW_P3_FEATURES=1` 才软放行）。
- Flutter 房间页据此 soft-hide 连麦/PK 菜单。  
- 连麦/PK 属 P3，见 [p3-p4-experimental](../product/p3-p4-experimental.md)。

## 7. 快速勾选

- [ ] `./scripts/deploy-test.sh`（或 `SKIP_DOGFOOD_SMOKE=1` + 手动冒烟）
- [ ] Flutter APK（emulator `10.0.2.2` / 真机 LAN）
- [ ] admin-web + h5-web：`pnpm build` + `preview`（端口不冲突）
- [ ] `gift-seed` → `10min-path` → `api-smoke` 均 PASS
- [ ] `GET /api/v1/meta` → `features.pk` / `features.cohost` **false**
- [ ] 人工：risk-accept / 设备矩阵 / OBS 周 / 缺陷会 / V-FL-2 URL / V-AD-1 — **未签不算关闭**

## 相关文档

- [go-live-local.md](./go-live-local.md) — 本地开播与 OBS
- [dogfood-cohort.md](./dogfood-cohort.md) — 队列、V-FL-1/2、签字
- [otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md) · [ws-1k-soak-risk-accept.md](./ws-1k-soak-risk-accept.md) — **unsigned** V-BE 草案
- [go-live-stage.md](./go-live-stage.md) — stage/prod 上线
- `scripts/dogfood-media.md` — 媒体面细节
