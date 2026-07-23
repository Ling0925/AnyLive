# P1 三轨并行任务拆分（Backend · Admin · Flutter）

> **目标：** 后端、管理后台、Flutter **同步推进**，缩短 P1 收口与 dogfood 周期。  
> **对照：** [06-P1进度评审与后续规划](./06-P1进度评审与后续规划.md) · [p1-git-split-plan](./p1-git-split-plan.md) · [p3-p4-experimental](./p3-p4-experimental.md) · [mvp-scope](./mvp-scope.md)  
> **状态：** draft · 2026-07-23  
> **硬约束：** 在人工明确要求前 **不** 代为 `git commit` / 开 PR；本文件是任务与接口契约，不是授权提交。

---

## 1. 怎么并行（先读这段）

### 1.1 三轨定义

| 轨道 | 代码边界 | 负责人画像 | 本地怎么跑 |
|---|---|---|---|
| **BE** 后端 | `backend/` · `contracts/` · `scripts/dogfood-*.sh` · `deploy/` | Rust / API | `cargo test --workspace` · `cargo run -p anylive-api` · `:8088` |
| **AD** 管理后台 | `apps/admin-web/` | Vue / 运营台 | `pnpm test` · `pnpm dev` · 默认调 `VITE_API_BASE` |
| **FL** Flutter | `apps/mobile/` | Dart 客户端 | `flutter test` · `flutter run` · `API_BASE` / flavor |

> H5（`apps/h5-web`）默认 **并入 FL 的「观众路径」联调**，或由同一 Web 同学兼；不单独占第四全职轨也可。

### 1.2 并行不等于乱序合并

| 层 | 规则 |
|---|---|
| **写代码** | 三轨 **同时** 开干（各改各的目录） |
| **合 main** | 仍建议 git 批次 **A → B → C → F**（见 split-plan）；**D/E 后置** |
| **联调** | 用 **已冻结的 API 契约** + 内存/测试栈 API，不互相等「整仓合完」 |
| **冲突文件** | `openapi.yaml` / `api/src/lib.rs` 由 **BE 主写**；AD/FL 只消费路径，PR 里不抢改路由注册 |

### 1.3 接口冻结（并行的前提）

三轨共用同一契约源：

| 契约 | 位置 | 谁改 | 谁消费 |
|---|---|---|---|
| OpenAPI | `contracts/openapi/openapi.yaml` | **BE** | AD / FL / 生成类型 |
| 错误码 | `contracts/errors/codes.yaml` | **BE** | 三端展示 |
| 环境变量 | `.env.example` · `deploy/.env.test` | **BE** 主写 | AD/FL 只读文档字段 |
| TS 生成 | `scripts/gen-openapi-ts.sh` → `apps/*/src/generated/` | BE 或 Web 跑脚本 | AD/H5 |
| 功能开关 | `FEATURE_*`（见 p3-p4-experimental） | **BE** 默认值 | AD/FL 只读 meta / 行为 |

**并行约定：**

1. 新路径先在 OpenAPI / 路由表 **补一行**（BE），再写 UI。  
2. AD/FL 可用 **mock 响应** 或本地 `cargo run` 联调，不阻塞对方。  
3. 破坏性改字段必须同日三轨群里广播，并改 OpenAPI。

---

## 2. 总览：泳道 × 阶段

```
时间 →
        Wave 0 收口准备     Wave 1 P1 主路径并行      Wave 2 出口验证
BE      拆 PR / CI 绿 ──►  会话安全播放对账 smoke ──► OTP/stage/缺陷
AD      路径 helper ──►    运维台/开播/对账/处置 ──►  运营预置/走查
FL      repo/session ──►   登录Feed房间播放钱包 ──►  真机矩阵/路径冒烟
        └──────── 每日 15min 三轨站会：契约 diff / 阻塞 ────────┘
```

| Wave | 时长（示意） | 三轨是否并行 | 产出 |
|---|---|---|---|
| **0** 准备 | 0.5–1 天 | 是 | 分支策略、契约基线、本地栈一键起 |
| **1** 功能并行 | 3–7 天 | **强并行** | P1 主路径可演示；A/B/C 可分别绿 |
| **2** 出口验证 | 3–7 天 | 是（偏验证） | 检查表签字项、tag 候选 |
| **3** 后置 | P1 后 | 可选 | D/E experimental |

---

## 3. Wave 0 — 准备（半天～1 天，可并行）

| ID | 轨道 | 任务 | 完成定义 |
|---|---|---|---|
| W0-BE-1 | BE | 确认 `./scripts/deploy-test.sh` 或 compose + `cargo run` 可用 | 健康检查 `:8088/health` 200 |
| W0-BE-2 | BE | OpenAPI 与当前路由 **diff 列表**贴到 PR 描述模板 | 无静默新路径 |
| W0-AD-1 | AD | `VITE_API_BASE` 指向本地 API；`pnpm test` 绿 | 登录页能打开 |
| W0-FL-1 | FL | `AppConfig` API base 可配；`flutter test` 绿 | 能打到本地 API |
| W0-ALL-1 | 全员 | 站会：认领下表 Wave 1 任务 ID | 无任务孤儿 |

---

## 4. Wave 1 — P1 主路径强并行（核心）

> 下列任务 **允许同一天三轨同时做**。依赖列写的是「联调依赖」，不是「必须等对方 commit 进 main」。

### 4.1 后端（BE）任务板

| ID | 任务 | 目录/入口 | 依赖 | DoD |
|---|---|---|---|---|
| **BE-1** | Auth 稳定：OTP、refresh、会话列表/吊销、生产守卫 | `auth/` `routes/auth.rs` `guards.rs` | — | 单测绿；dogfood 登录路径通 |
| **BE-2** | Rooms + Media：start/stop、publish/play、SRS webhook | `routes/rooms` `media/` `webhooks` | — | smoke 含 publish URL |
| **BE-3** | Chat + 限流 + 敏感词 + realtime token/publish | `routes/chat` `realtime/` | — | 发消息 201；mute 后 403 |
| **BE-4** | Wallet/Gifts：ledger、幂等、live-only | `wallet/` `routes/wallet` | — | 双发同 key 不双扣 |
| **BE-5** | Pay **沙箱 only**（mock 建单/入账/对账） | `pay/` `routes/pay` | BE-4 | sandbox-complete 入账 |
| **BE-6** | Admin API：ban/mute/force-close/gifts/reports/reconcile | `routes/admin*` | BE-1 | smoke 处置路径通 |
| **BE-7** | Feed/search/presence（P1 边缘） | `feed` `search` `presence` | BE-2 | 列表/搜索 200 |
| **BE-8** | Compliance：legal/export/delete | `routes/compliance` | BE-1 | export 有实质字段 |
| **BE-9** | Dogfood 脚本维护 | `scripts/dogfood-*.sh` | BE-1…6 | api-smoke / 10min-path 绿 |
| **BE-X** | **后置** interactive/PK/events/push/oauth | 见 D/E | 禁止挡 Wave1 | 默认 FEATURE 关 |

**BE 并行内部建议：** 一人守 `lib.rs`/OpenAPI 合并；域逻辑可分人（auth / rooms / wallet）。

### 4.2 管理后台（AD）任务板

| ID | 任务 | 目录/入口 | 依赖（契约） | DoD |
|---|---|---|---|---|
| **AD-1** | OTP 登录壳 + token 内存持有 | `admin-web` App / `admin.ts` | `POST otp/*` | 能拿 access_token |
| **AD-2** | 房间表 + 状态刷新 | 房间模块 | `GET /rooms` | 列表展示 live/idle |
| **AD-3** | 开播信息：推流 URL/密钥展示与复制 | 开播/房间详情 | `media/publish` | 运营能抄给主播 OBS |
| **AD-4** | 处置：ban / mute / unmute / force-close | 处置模块 | admin POST 族 | 点按钮有 2xx/错误提示 |
| **AD-5** | 礼物目录列表 + upsert（若 API 已有） | 礼物模块 | `admin/gifts` | 能加一条测试礼物 |
| **AD-6** | 举报队列列表 + resolve | 举报模块 | `admin/reports` | 能点处理 |
| **AD-7** | 资金：对账按钮 + 关单（可选） | 总览 | `admin/wallet/reconcile` 等 | 展示结果 JSON/摘要 |
| **AD-8** | HLS 预览（可选） | 预览 | play URL + hls.js | live 房能挂播放器 |
| **AD-9** | 路径 helper 单测 | `admin.spec.ts` | — | `pnpm test` 绿 |

**AD 与 BE 解耦技巧：** AD-1…4 可先用固定 mock JSON 画 UI；BE-6 一好即切真 API。

### 4.3 Flutter（FL）任务板

| ID | 任务 | 目录/入口 | 依赖（契约） | DoD |
|---|---|---|---|---|
| **FL-1** | 会话持久化 + 启动恢复 + Logout | `session_store` `app.dart` | token 形状 | 杀进程仍登录 |
| **FL-2** | 登录 + 隐私/条款 + 年龄门 | `login_page` | otp + PATCH /me | 未勾年龄不能进 |
| **FL-3** | Feed / 热门·关注列表 | `feed_page` | `feed/hot|following` | 可下拉进房 |
| **FL-4** | 房间列表 + Go live + publish 对话框 | `room_list_page` | rooms + media/publish | 能展示 OBS 信息 |
| **FL-5** | 房间页：状态、聊天、礼物、余额、结束态 | `room_page` | messages/gifts/wallet | 非 live 禁送 |
| **FL-6** | 播放：media_kit / StreamPreview | `player/` | play URL | 有 URL 可预览或明确降级文案 |
| **FL-7** | 关注 + 举报入口 | social/reports repo + UI | follow/reports | 菜单可点且有反馈 |
| **FL-8** | 资料：昵称/年龄/导出/删号 | `profile_page` | me/export/delete | 导出可复制 |
| **FL-9** | 钱包页：余额/ledger/沙箱买币 | `wallet_page` pay_repo | wallet + pay sandbox | 沙箱入账后余额变 |
| **FL-10** | 可选 Centrifugo WS + HTTP 轮询回退 | `realtime/` | realtime/token | 无 WS 时仍能聊 |
| **FL-11** | widget/repo 单测补齐 | `test/` | — | `flutter test` 绿 |
| **FL-X** | **后置** 连麦/PK UI、埋点、push | interactive/events | FEATURE 关 | 不进 Demo 脚本 |

**FL 与 BE 解耦技巧：** repository 层先用 `http` + 假 baseUrl 单测；真机联调只改 config。

### 4.4 同日并行对照（减少互相等待）

| 时刻主题 | BE 做 | AD 做 | FL 做 |
|---|---|---|---|
| 登录 | BE-1 | AD-1 | FL-1 · FL-2 |
| 开播/看播 | BE-2 | AD-2 · AD-3 · AD-8 | FL-4 · FL-5 · FL-6 |
| 聊天礼物 | BE-3 · BE-4 | （只读房间流水可选） | FL-5 · FL-10 |
| 运营处置 | BE-6 | AD-4 · AD-5 · AD-6 | （被 ban 后体验回归） |
| 钱包沙箱 | BE-5 | AD-7 | FL-9 |
| 合规 | BE-8 | （文案链到 legal） | FL-8 |
| 收口脚本 | BE-9 | AD-9 | FL-11 |

---

## 5. Wave 2 — 出口验证（仍可并行）

| ID | 轨道 | 任务 | DoD |
|---|---|---|---|
| V-BE-1 | BE | 真 OTP 通道（ESP/HTTP）或书面接受 dev-only | 非仅口头 |
| V-BE-2 | BE | stage 配置 / 1k soak 或风险接受书 | 报告落 `reports/` |
| V-AD-1 | AD | 运营预置：礼物目录、admin 账号、演示脚本走查 | 15 分钟演示可跟 |
| V-FL-1 | FL | 设备矩阵：中端 Android + 近两代 iPhone | 填模板 |
| V-FL-2 | FL | 10 分钟真人路径录屏（可与 OBS 同学一起） | 链接进 dogfood 纪要 |
| V-ALL-1 | 全员 | 缺陷会：无开放 P0 | 纪要 |
| V-ALL-2 | 全员 | git 按 [p1-git-split-plan](./p1-git-split-plan.md) 收敛 | 可打 tag 候选 |

---

## 6. 与 git 批次（A–F）的映射

三轨并行写代码时，**落库仍按批次**，避免 150 文件单 PR：

| 三轨任务 | 建议 git 批次 | 说明 |
|---|---|---|
| BE-1…4,7,8 · FL-1…8,10,11 · 契约/smoke | **A** | P1 加固主包 |
| BE-5 · FL-9 · pay 契约 | **B** | Pay 沙箱 |
| AD-1…9 · admin 迁移/seed | **C** | Admin（可与 A 分 commit 同窗口） |
| 文档/runbook/本文件 | **F** | 勾选不得超前 main |
| BE-X · FL-X | **D/E** | experimental，后置 |

详见：[p1-git-split-plan.md](./p1-git-split-plan.md)。

---

## 7. 冲突与「接口所有权」

| 资源 | 所有者 | 他人规则 |
|---|---|---|
| `backend/crates/api/src/lib.rs` 路由注册 | **BE** | AD/FL 禁止改 |
| `contracts/openapi/openapi.yaml` | **BE** | 变更当日通知；生成物可 AD 跑脚本 |
| `apps/admin-web/src/App.vue` | **AD** | BE 不改 UI |
| `apps/mobile/lib/features/**` | **FL** | BE 不改 Dart UI |
| `room_page.dart` 内 PK 菜单 | **FL** | 用 flag 隐藏；逻辑归 D 批 |
| `scripts/dogfood-*.sh` | **BE** 主 | 路径变更通知 AD/FL |
| `FEATURE_*` 默认值 | **BE** | 客户端不硬编码「已支持 PK」 |

**高冲突文件处理：** 见 split-plan「混装 / 高冲突」表；优先 `git add -p` 或叠小 commit。

---

## 8. 联调清单（三轨对齐用）

每日站会只对这 8 条（有阻塞就升级）：

1. 本地 API 是否可登录（dev OTP 规则是否变）  
2. `media/publish` / `play` 字段是否变名  
3. 聊天 POST/GET 与 mute 行为  
4. 送礼 body（`gift_id` / `client_request_id` / `receiver_id`）  
5. Admin 处置 API 是否 401/403 语义变了  
6. Pay sandbox 是否仅测试栈开启  
7. 是否有人误开 `FEATURE_PK` / `FEATURE_COHOST`  
8. OpenAPI 是否有未广播的 breaking change  

**推荐联调命令（示意）：**

```bash
# BE
cd backend && cargo test --workspace && cargo run -p anylive-api

# 栈（可选）
./scripts/deploy-test.sh   # 或 compose + API

# AD
cd apps/admin-web && pnpm test && pnpm dev

# FL
cd apps/mobile && flutter test && flutter run
```

---

## 9. 明确不做（并行期）

| 不做 | 原因 |
|---|---|
| 把连麦/PK 当 P1 Demo 主路径 | mvp-scope 整期不做；见 experimental |
| 生产真收款 / 多通道深接入 | P2；本阶段仅 mock 沙箱 |
| 等「整仓合完」再写 UI | 违背并行目标；用契约 + 本地 API |
| 单 PR 清掉全部 dirty tree | review/回滚失败 |
| 伪造设备矩阵 / 真人 OBS 通过 | 出口诚信 |

---

## 10. 角色与 RACI（可填人名）

| 任务族 | BE | AD | FL | TL |
|---|---|---|---|---|
| API / 契约 | **A** | C | C | I |
| Admin 运维台 | C | **A** | I | I |
| Flutter 主路径 | C | I | **A** | I |
| Dogfood 脚本 | **A** | C | C | I |
| 真机矩阵 | I | I | **A** | R |
| OBS 真人周 | C | C | C | **R** |
| 范围/签字 | C | C | C | **A** |

A=执行 · R=负责结果 · C=协商 · I=知会

---

## 11. 进度看板（复制到 Issue / 表格）

| ID | 轨道 | 状态 | 负责人 | 目标日 | 备注 |
|---|---|---|---|---|---|
| BE-1 | BE | todo | | | |
| BE-2 | BE | todo | | | |
| BE-3 | BE | todo | | | |
| BE-4 | BE | todo | | | |
| BE-5 | BE | todo | | | |
| BE-6 | BE | todo | | | |
| BE-7 | BE | todo | | | |
| BE-8 | BE | todo | | | |
| BE-9 | BE | todo | | | |
| AD-1 | AD | todo | | | |
| AD-2 | AD | todo | | | |
| AD-3 | AD | todo | | | |
| AD-4 | AD | todo | | | |
| AD-5 | AD | todo | | | |
| AD-6 | AD | todo | | | |
| AD-7 | AD | todo | | | |
| AD-8 | AD | todo | | | |
| AD-9 | AD | todo | | | |
| FL-1 | FL | todo | | | |
| FL-2 | FL | todo | | | |
| FL-3 | FL | todo | | | |
| FL-4 | FL | todo | | | |
| FL-5 | FL | todo | | | |
| FL-6 | FL | todo | | | |
| FL-7 | FL | todo | | | |
| FL-8 | FL | todo | | | |
| FL-9 | FL | todo | | | |
| FL-10 | FL | todo | | | |
| FL-11 | FL | todo | | | |
| V-ALL-1 | 全员 | todo | | | 缺陷会 |
| V-ALL-2 | 全员 | todo | | | git 收敛 |

状态建议：`todo` / `doing` / `blocked` / `done`。

---

## 12. 完成定义（Wave 1 集体）

当且仅当：

1. BE-9 脚本绿（或已知 flaky 有人认领）；  
2. AD 能登录并完成：看房 → 抄推流信息 → 禁言/强关 各一次；  
3. FL 能完成：登录 → Feed/列表 → 进房 → 发言 → 送礼（测试币）→ 见结束态；  
4. 三轨约定的 OpenAPI 无未同步 breaking change；  
5. PK/连麦未出现在 Demo 脚本。  

→ 可进入 Wave 2 出口验证，并按 split-plan 推进 **A/B/C** 合入。

---

## 13. 相关链接

- 进度评审与闸门：[06-P1进度评审与后续规划.md](./06-P1进度评审与后续规划.md)  
- Git 文件归批：[p1-git-split-plan.md](./p1-git-split-plan.md)  
- P3/P4 非退出：[p3-p4-experimental.md](./p3-p4-experimental.md)  
- 实现状态：[p1-status.md](./p1-status.md)  
- 本地开播：[../runbooks/go-live-local.md](../runbooks/go-live-local.md)  

---

*更新本表时请改「状态」行日期，并与 `git status` / OpenAPI 保持一致。*
