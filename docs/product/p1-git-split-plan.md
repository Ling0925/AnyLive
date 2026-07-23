# P1 git 拆分草案（Stage 0 · plan 06）

> **状态：draft**（`06-P1进度评审与后续规划.md` §8.2 仓库收口）  
> **对照：** [06-P1进度评审](./06-P1进度评审与后续规划.md) · [p1-status](./p1-status.md) · [p3-p4-experimental](./p3-p4-experimental.md)  
> **工作区规模（盘点时点 2026-07-23）：** ~152 路径（约 90 modified + 62 untracked）；以当前 `git status` 为准，数字会漂。  
> **顶层分布（约）：** backend ~61 · apps ~54 · docs ~20 · scripts ~10 · contracts ~3 · 其他（env/ci/README/deploy）

---

## 硬约束（读完再动手）

1. **禁止把整个 monorepo 脏树塞进单一巨型 PR / 单一巨型 commit。**  
   单 PR 多阶段混装会让 review 失效、回滚困难，并继续放大「文档完成态 > 可合并 `HEAD`」的双源风险（plan 06 · R1 / R9 / R11）。
2. 合并顺序固定为 **A → B → C，然后 F**；**D / E 后置**（experimental，非 P1 退出）。  
   F（纯文档）可与 A **并行**起草，但 **合入时机**仍建议在 A/B/C 之后，避免勾选超前于已合并代码。  
   **允许**先落「主题小 commit」（如结束态 UX、risk-accept 草案）再按批开 PR，只要每 commit 可独立 `revert` 且不混 D/E 默认开。
3. 混装 / 高冲突文件（`lib.rs` / `state.rs` / OpenAPI / `room_page.dart` / Admin `App.vue`）按「主意图」归批；必要时 `git add -p` 或叠两个小 commit，**不要**为省事回退到 monorepo 单 PR。
4. 用户若要求「提交好 git」：按主题 commit，**不要**一次 `git add -A`。

---

## 原则

| # | 原则 |
|---|---|
| 1 | **先收敛、再扩阶段**；未拆分的大工作区视为不可发布。 |
| 2 | 每批 PR 标题/描述引用本表批次字母（`batch-A` …），并写清「P1 出口相关 / experimental」。 |
| 3 | CI 在拆分 PR 上全绿：`cargo` / `flutter test` / vitest / contracts。 |
| 4 | D/E 代码可保留，但必须默认关开关、文档标明 **非 P1 退出条件**（见 [p3-p4-experimental](./p3-p4-experimental.md)）。 |
| 5 | 本草案 **不** 伪造设备矩阵 / soak / 真人 OBS 通过记录；risk-accept 文件 **未签字 ≠ 出口关闭**。 |

---

## 批次总表

| 批次 | 主题 | 约计路径 | 优先级 | 合并顺序 |
|---|---|---:|---|---|
| **A** | P1 加固：安全、会话、播放、可选 WS、对账、smoke、合规 UI、契约/CI、结束态 | ~90 | **P0** | **1** |
| **B** | Pay 沙箱（mock + webhook 形状 + Flutter 钱包） | ~10 | P0/P1 | **2** |
| **C** | Admin 运维增强（面板、seed、角色迁移） | ~5 | P1 | **3** |
| **F** | 纯文档 / runbook / 产品规划 / risk-accept 草案 | ~30 | 可与 A 并行起草 | **4**（A/B/C 后合入，或小文档 commit 可先） |
| **D** | P3 interactive / PK / LiveKit（experimental） | ~8 | **后置** | 独立 PR |
| **E** | P4 events / push / oauth / recording 脚手架（experimental） | ~20 | **后置** | 独立 PR |

**推荐落地顺序：** `A → B → C → F`，然后（可选）`D`、`E`。

### 2026-07-23 建议优先落盘的「小主题 commit」（可先于整批 A PR）

| 主题 commit | 主要路径 | 说明 |
|---|---|---|
| end-state idle vs closed | `apps/mobile/lib/player/*`、`features/rooms/room_page.dart`、`home/feed/room_list` userId；`apps/h5-web/src/lib/share*`、`App.vue` loadRoom | 客户端结束态；不混 pay/PK |
| risk-accept drafts | `docs/runbooks/otp-dev-only-risk-accept.md`、`ws-1k-soak-risk-accept.md`；soak-status 交叉链 | **未签字** |
| status honesty | `docs/product/p1-status.md`、`06-…`、`go-live-local.md` media_kit、`dogfood-cohort` 预检 | 不伪造成功 |

其余仍按 A/B/C/D/E/F 大表，**禁止**一次 add 全树。

---

## 批次明细

### A — P1 加固（优先 · ~90）

安全 / 会话 / 播放 / 可选 WS / 对账 / smoke / 合规 / 功能开关纪律 / 契约与 CI / 结束态。

**后端 API / 域**

| 组 | 路径（可再拆 commit） |
|---|---|
| 入口与守卫 | `backend/crates/api/src/main.rs`, `lib.rs`, `guards.rs`, `features.rs`, `state.rs`, `tracing_init.rs`, `presence.rs` |
| 路由 | `routes/mod.rs`, `auth.rs`, `chat.rs`, `compliance.rs`, `feed.rs`, `presence.rs`, `rooms.rs`, `search.rs`, `system.rs`, `wallet.rs`, `admin.rs` |
| Auth / DB | `backend/crates/auth/src/{lib,service,store}.rs`；`db/src/{lib,users,refresh,profile,rooms,social,wallet,moderation}.rs` |
| 媒体 / 社交 / 钱包 / 审核 | `backend/crates/media/`, `social/`, `wallet/`, `moderation/`（crate 源） |
| 迁移 | `backend/migrations/009_profile_region.sql` |
| 依赖锁 | `backend/Cargo.lock`；`backend/crates/api/Cargo.toml`；`backend/crates/media/Cargo.toml` |

**客户端**

| 组 | 路径 |
|---|---|
| Flutter API / 配置 | `apps/mobile/lib/api/{auth,profile,realtime,rooms}_repository.dart`, `session_store.dart`；`config/app_config.dart`；`app.dart`；`navigation/app_routes.dart` |
| Flutter UI | `features/auth/login_page.dart`, `feed/feed_page.dart`, `home/home_page.dart`, `profile/profile_page.dart`, `rooms/{room_page,room_list_page}.dart` |
| 播放 / 实时 | `player/{stream_preview,hls_player_logic}.dart`；`realtime/centrifugo_chat.dart` |
| 依赖 | `apps/mobile/pubspec.yaml`, `pubspec.lock` |
| 测试 | `apps/mobile/test/` 下 app_config / routes / centrifugo / profile / realtime / room / session / stream_preview / hls_player_logic / widget 等 |

**H5 / Admin 壳（与 A 主路径相关）**

| 组 | 路径 |
|---|---|
| H5 | `apps/h5-web/package.json`, `src/App.vue`, `src/lib/{chatApi,realtime,share}.{ts,spec.ts}`, `src/generated/openapi.d.ts` |
| Admin 生成类型 / package | `apps/admin-web/package.json`, `src/generated/openapi.d.ts`（**面板逻辑归 C**） |

**契约 / 脚本 / 环境**

| 组 | 路径 |
|---|---|
| OpenAPI / webhook 形状 | `contracts/openapi/openapi.yaml`；`contracts/webhooks/srs.on_publish.v1.json`；`scripts/validate-contracts.py`；`scripts/gen-openapi-ts.sh` |
| Dogfood / loadtest | `scripts/dogfood-{api-smoke,10min-path,cohort-seed,gift-seed}.sh`；`scripts/loadtest/{README.md,gift-tps-baseline.sh,ws-centrifugo-load.py}` |
| CI / env | `.github/workflows/ci.yml`；`.env.example`；`deploy/.env.test` |

> **说明：** `features.rs` 同时承载 P3 开关默认值，**主意图归 A**（P1-safe 默认 OFF）；D 批只叠 interactive 路由与客户端。

---

### B — Pay 沙箱（~10）

| 区域 | 路径 |
|---|---|
| 后端 | `backend/crates/pay/src/lib.rs`；`backend/crates/api/src/routes/pay.rs`；`backend/crates/db/src/pay.rs` |
| Flutter | `apps/mobile/lib/api/pay_repository.dart`；`features/wallet/wallet_page.dart`；`test/pay_repository_test.dart`；`test/wallet_page_test.dart` |
| 契约 | `contracts/webhooks/README.md`；`pay.hmac.v1.json`；`pay.mock.v1.json` |

**叙事边界：** 本批只保证 **mock / sandbox 建单 + 入账 + 对账路径**。Stripe/IAP/Jeepay 等多通道适配器若已在同文件，可同 PR 合入但 **PR 描述标明「P2 形状，非 P1 真收款」**；必要时再拆子批。

---

### C — Admin 运维增强（~5）

| 区域 | 路径 |
|---|---|
| 面板 | `apps/admin-web/src/App.vue`；`src/lib/admin.ts`；`src/lib/admin.spec.ts` |
| 角色 / seed | `backend/migrations/010_admin_roles.sql`；`scripts/seed-admin-local.sh` |

若 `App.vue` 中 gate 文案与 A 强耦合且难拆，允许 **与 A 同 PR**，但 commit 消息仍分主题（`feat(admin): …` vs `feat(auth): …`）。

---

### D — P3 interactive / PK / LiveKit（后置 · experimental · ~8）

| 区域 | 路径 |
|---|---|
| 后端 | `backend/crates/api/src/interactive.rs`；`routes/interactive.rs`；`backend/crates/domain/src/interactive.rs`；`domain/src/lib.rs` |
| Flutter | `apps/mobile/lib/api/interactive_repository.dart`；`test/interactive_repository_test.dart` |

**要求：** 独立 PR；标题含 `experimental`；默认 `FEATURE_PK=0` / `FEATURE_COHOST=0`；**禁止**写入 P1 出口检查表。关联 runbook / 说明见 F 中 `livekit-stage.md` 与 `p3-p4-experimental.md`（文档可先合 F，代码仍后置）。

---

### E — P4 增长 / push / oauth / 录制脚手架（后置 · ~20）

| 组 | 路径（摘要） |
|---|---|
| 埋点 / 创作者 | `backend/crates/api/src/{analytics,invite,oauth}.rs`；`routes/{events,creator,avatar,push,recording}.rs`；`push.rs`, `push_delivery.rs`, `recording.rs`, `object_storage.rs` |
| 实时 NATS 接线 | `backend/crates/realtime/src/{lib,nats}.rs` |
| 迁移 / 契约事件 | `backend/migrations/008_avatar_recording.sql`；`contracts/events/gift.sent.v1.json` |
| Flutter | `apps/mobile/lib/api/{events,push}_repository.dart`；`test/events_repository_test.dart` |

**要求：** 独立 PR 或「experimental scaffold」标签；不阻塞 P1 tag。

---

### F — 纯文档 / runbook / 商店文案（~30）

| 组 | 路径（摘要） |
|---|---|
| 产品规划 | `docs/product/{README,01-阶段与里程碑,02-WBS与排期,06-P1进度评审与后续规划,mvp-scope,p1-status,p3-p4-experimental,event-dictionary,p1-git-split-plan,p1-parallel-tracks}.md` |
| 架构 | `docs/architecture/payment-channels.md` |
| Runbooks | `docs/runbooks/{go-live-local,go-live-stage,livekit-stage,dogfood-cohort,backup-restore,otel,report-sla,slo-alerts,store-internal,otp-dev-only-risk-accept,ws-1k-soak-risk-accept}.md` |
| 报告状态 | `reports/ws-1k-soak-status-20260722.md`（及 templates；**不**伪造 measured 通过） |
| 商店文案 | `apps/mobile/store/{README,listing-en,listing-zh}.md` |
| 根 README | `README.md` |

**合入纪律：** F 中对功能的 `[x]` 勾选必须与 **已合并** 代码一致；未进 `main` 的项标 `unreleased` 或保持未勾；risk-accept **未签字**不得写成出口通过。

---

## 混装 / 高冲突文件（~22 critical）

| 路径 | 建议 |
|---|---|
| `backend/crates/api/src/lib.rs` / `state.rs` / `routes/mod.rs` | 主意图进 **A**；D/E 路由注册可叠小 commit |
| `backend/crates/api/src/features.rs` | **A**（默认关 + 文档） |
| `backend/crates/pay/src/lib.rs` | **B**（支付主实现） |
| `contracts/openapi/openapi.yaml` | 随首个消费批（通常 **A**），B/D/E 路径增量 commit |
| `apps/mobile/lib/features/rooms/room_page.dart` | 播放/会话/结束态进 **A**；连麦/PK 菜单用 flag 软隐藏，块可后拆 **D** |
| `apps/admin-web/src/App.vue` | **C**（或与 A 同 PR、分 commit） |
| `backend/crates/api/src/{presence,invite,object_storage,recording}.rs` 等 | presence/search/system → **A**；invite/oauth/push/recording → **E** |
| 商店 listing | **F**（文案）；不绑功能 PR |

---

## 推荐合并顺序（执行清单）

```
0. （可选）主题小 commit：end-state / risk-accept / status honesty — 可独立 revert
1. A  — CI 全绿；P1 主路径可回滚点
2. B  — mock pay only 叙述；无生产真收款
3. C  — Admin 增强（若未并入 A）
4. F  — 文档对齐「已合并」勾选（起草可并行 A）
5. D  — experimental PR；默认关开关
6. E  — experimental PR；不进 P1 tag 条件
```

每步完成后才进入下一步；**禁止**为「一次清 dirty tree」合并为 monorepo 单 PR。

---

## 风险：monorepo 单 PR

| 风险 | 说明 | 缓解 |
|---|---|---|
| Review 失效 | 150+ 文件跨 auth/pay/PK/docs，无法按主题否决 | 强制批次字母 + 独立 PR |
| 回滚成本 | 一处回归拖垮全部 | 每批可独立 `revert` |
| 文档双源 | `p1-status` 勾选指向未合并代码 | F 后置合入；勾选跟 `main` |
| CI 噪声 | 单 PR 失败无法定位域 | 每批各自绿再建下一个 |
| 范围叙事污染 | P3/P4 被当成 P1 完成 | D/E 标题 experimental + 默认 OFF |

---

## 明确不做（本草案执行期）

- 一次 `git add -A` / monorepo 巨型 commit 清树  
- 把 D/E 勾成 P1 完成或写入 dogfood 成功标准  
- 伪造设备矩阵 / 15min soak / 真人 OBS 周记录  
- 把 **未签字** risk-accept 写成出口关闭  
- 生产真收款、公开上架、把 experimental 默认打开  

---

## 下一步（人工 + 主题 commit）

1. 审阅本表路径归类与批次边界，反馈改 batch 归属。  
2. 优先落地「小主题 commit」表（结束态 / risk-accept / status），再从 A 开分支或 `git add -p`。  
3. 每批 PR 描述引用 `batch-A`… 与 plan 06 §8.2。  
4. A/B/C 合入后更新 `p1-status` / 里程碑勾选（F），再考虑 D/E。  
5. P1 出口检查表（plan 06 §8.3）在 git 收敛 + 签字项后单独会签。
