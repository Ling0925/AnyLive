# Flutter 移动端 UX 设计：YouTube 风格直播体验

> **产品：** AnyLive（海外秀场直播）  
> **端：** `apps/mobile`（Flutter）  
> **参考：** YouTube App / YouTube Live 的信息架构与交互范式（**非**像素级抄袭、**非**使用 YouTube 商标资源）  
> **约束：** 对齐 [mvp-scope](./mvp-scope.md) P1 必须能力；连麦/PK/短视频 **整期不做**  
> **对照：** [p1-parallel-tracks](./p1-parallel-tracks.md) FL 任务 · [p1-status](./p1-status.md) · [h5-youtube-ux](./h5-youtube-ux.md) H5 RoomWatch 镜像  
> **状态：** draft · 2026-07-23  
> **读者：** 产品 / Flutter / 设计 / TL

---

## 1. 设计目标

### 1.1 为什么参考 YouTube

| YouTube 已教育用户的习惯 | AnyLive 收益 |
|---|---|
| 底部 Tab 找内容 / 我的 | 降低学习成本 |
| 大封面卡片 + 标题 + 主播行 | 快速扫列表选直播 |
| 点进即播，聊天在视频下方或侧栏感 | 看播为主、互动为辅 |
| 红点 LIVE 徽章、观看人数 | 直播状态一眼可辨 |
| 关注、分享、更多（举报等）聚合 | 社交与合规入口不散乱 |
| 深色观影、手势返回 | 沉浸、少误触 |

### 1.2 设计原则

1. **Watch-first：** 首屏与进房路径优先「看见画面」，其次聊天/礼物。  
2. **Familiar IA：** 信息架构贴近 YouTube Mobile（Home / 搜索 / 创作入口 / 订阅感 / 我的），语义换成直播域。  
3. **Live-native 增量：** 在 YouTube 壳上叠加秀场必需：礼物栏、余额、开播/OBS、结束态。  
4. **P1 可落地：** 不引入短视频 For You 算法、不引入 Shorts 全屏流、不引入 PK UI 主路径。  
5. **可达与合规：** 年龄/隐私、举报、导出删除在「我的」与房间「更多」可发现。  

### 1.3 成功标准（体验向）

| 指标 | P1 目标 |
|---|---|
| 新用户找到正在播的房间 | ≤ 2 次点击（从冷启动到进房） |
| 进房到看到播放器区域 | ≤ 1s 出骨架；有流则自动尝试播放 |
| 发言 / 送礼 | 不离开观看上下文（底栏或半屏 sheet） |
| 主播拿到 OBS 信息 | 从「创作」入口 ≤ 3 步 |
| 视觉一致性 | 统一 Token（色/字/圆角/间距），禁止每页一种风格 |

---

## 2. 竞品映射：YouTube → AnyLive

### 2.1 导航映射

| YouTube Mobile | AnyLive P1 | 说明 |
|---|---|---|
| Home（推荐/订阅混合流） | **Home** 直播发现流 | 热门 + 分区条；P1 无强推荐模型，用 `/feed/hot` |
| Shorts | **不做** | mvp-scope 明确不做短视频 |
| Subscriptions | **Following**（关注中的直播） | `/feed/following`；无直播时空态引导去 Home |
| Library / You | **You（我的）** | 资料、钱包、合规、会话、设置 |
| 顶部搜索 | **Search** | `GET /search`；P1 可简化为房间/用户关键词 |
| 创建（相机） | **Go Live** | 创建房间 + start + 展示 publish（OBS）；非 App 内采集推流也可 |

**底部 Tab 建议（5 或 4）：**

```
[ Home ]  [ Following ]  [  ● Go Live  ]  [ Inbox* ]  [ You ]
```

| Tab | P1 | 备注 |
|---|---|---|
| Home | **必须** | 发现 |
| Following | **必须** | 关注中的 live |
| Go Live | **必须** | 中央凸起或普通 Tab + 主色按钮 |
| Inbox | **P1 可隐藏** | 无站内信；有系统通知再开；避免空壳 |
| You | **必须** | 账号与资产 |

P1 若只做 **4 Tab**：`Home | Following | Go Live | You`，搜索放 Home 顶栏（更接近 YouTube）。

### 2.2 功能贴近度矩阵

| 能力 | YouTube 参考行为 | AnyLive P1 实现 | 贴近度 |
|---|---|---|---|
| 浏览列表 | 竖滑卡片、16:9 封面 | Feed 卡片；封面可占位色+标题 | 高 |
| LIVE 标识 | 红底 LIVE | 状态 chip `LIVE` / 人数 | 高 |
| 进房播放 | 点卡片进播放器 | `RoomPage` + HLS / media_kit | 高 |
| 聊天 | Live chat 列表 | HTTP 历史 + 可选 WS；UI 做成 chat panel | 中→高 |
| 点赞 | 超级感谢/点赞 | 可用 presence likes API 做轻量按钮 | 中 |
| 关注 | Subscribe | follow/unfollow | 高 |
| 分享 | Share sheet | 系统分享 + H5 深链（若有） | 中 |
| 举报 | Report | 房间更多菜单 → reports API | 高 |
| 礼物/打赏 | Super Chat / 会员 | **礼物栏 + 余额**（秀场差异化主功能） | 特有 |
| 钱包充值 | 无直接对标 | Wallet 页 + 沙箱买币 | 特有 |
| 开播 | 移动直播/OBS 外链 | Go Live + publish 对话框（OBS） | 中（P1 偏 OBS） |
| 结束态 | 直播结束页 | 非 live 明确 banner，禁聊禁送 | 高 |
| 推荐算法 | 强 | P1 热门排序即可 | 低（可接受） |
| Shorts / 电商 | 有 | **不做** | — |
| 连麦 PK | 无/少 | **P1 不做**（开关默认关） | — |

### 2.3 必须「像 YouTube」vs 必须「像秀场」

**像 YouTube（体验壳）：**

- 底 Tab + 顶搜索  
- 大图卡片流  
- 播放器优先布局  
- 主播行（头像、名、粉丝向文案）  
- 更多菜单（⋯）收敛次要操作  
- 深色主题观影  

**像秀场（业务核）：**

- 礼物横滑栏 + 价格 + 送礼动效（可简化）  
- 余额 / 充值入口靠近礼物  
- 开播推流信息（OBS）  
- 运营强关 / 禁言后的客户端提示  

---

## 3. 信息架构（IA）

```
App
├── AuthStack（未登录）
│   ├── Welcome / Login（OTP）
│   └── 合规：隐私 / 条款 / 年龄门
│
└── MainShell（已登录 · BottomNavigation）
    ├── HomeTab
    │   ├── TopBar: Logo · Search · (可选通知)
    │   ├── ChipRow: Hot | 分类*(P2)
    │   └── LiveCardList → RoomWatch
    ├── FollowingTab
    │   └── LiveCardList（空态：去发现）
    ├── GoLiveTab / Sheet
    │   ├── 创建标题 → create+start
    │   ├── PublishInfo（RTMP/key 复制）
    │   └── 我的直播间状态 / 停播
    └── YouTab
        ├── 头像昵称 · 编辑资料
        ├── 钱包与流水
        ├── 创作者数据*(可选只读)
        ├── 会话管理 / 登出
        └── 隐私导出 · 删号 · 关于
```

**房间全屏栈（从卡片 push）：**

```
RoomWatch
├── PlayerStage（16:9 或 全宽）
├── MetaRow（标题 · LIVE · 在线）
├── ChannelRow（头像 · 名 · Follow · ⋯）
├── ChatPanel（可滚动）
├── Composer（输入 + 发送）
└── GiftDock（横滑礼物 · 余额 · 充值入口）
```

---

## 4. 关键页面线框（文字线框）

### 4.1 Home（YouTube Home 感）

```
┌─────────────────────────────────────┐
│ [● AnyLive]          [🔍]  [avatar] │  ← 顶栏紧凑
├─────────────────────────────────────┤
│ (Hot)  Following*                    │  ← 横向 chip；*可跳 Following Tab
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │     16:9 封面 / 占位渐变         │ │
│ │     ┌────┐                       │ │
│ │     │LIVE│  1.2K watching        │ │
│ └─────────────────────────────────┘ │
│  [头像] 房间标题两行截断…            │
│         主播名 · 刚开播               │
├─────────────────────────────────────┤
│ … 下一张卡片 …                       │
└─────────────────────────────────────┘
│ Home  Following  [●]  You            │
```

**交互：**

- 下拉刷新 → 重新拉 `/feed/hot`  
- 点击卡片 → `RoomWatch`（带 hero 可选）  
- 长按卡片（P2）→ 不感兴趣；P1 可省略  
- 搜索图标 → Search 页  

### 4.2 RoomWatch（Live 观看，偏 YouTube Live + 秀场）

```
┌─────────────────────────────────────┐
│ ←                                 ⋯ │
│ ┌─────────────────────────────────┐ │
│ │         VIDEO / HLS PLAYER      │ │  ← 固定顶区，黑底
│ │         (双击点赞* · 单击显控)   │ │
│ └─────────────────────────────────┘ │
│ 标题 · LIVE · 128 online             │
│ [头像] HostName          [Follow]    │
├─────────────────────────────────────┤
│ Chat                                │
│  UserA: hello                       │
│  UserB: 🔥                          │
│  …自动滚到底…                        │
├─────────────────────────────────────┤
│ [  说点什么…              ] [Send]  │
│ [🌹1] [🚀10] [👑99] …   余额 120 ＋ │  ← 礼物 dock
└─────────────────────────────────────┘
```

**结束态（status ≠ live）：**

- Player 区遮罩：「直播已结束」  
- 禁发送 / 禁送礼  
- 主按钮：返回 Home  

**更多菜单（⋯）：**

- 分享  
- 举报房间  
- 复制直播链接  
- （主播本人）停播 / 查看推流信息  

### 4.3 Go Live（创作入口，对齐「发布」心智）

```
┌─────────────────────────────────────┐
│  Go Live                            │
│  房间标题  [________________]       │
│  [ 开始直播 ]                       │
│                                     │
│  推流方式：OBS（推荐）               │
│  服务器：rtmp://…        [复制]     │
│  串流密钥：****          [复制]     │
│  说明：密钥勿分享；停播后失效*       │
│                                     │
│  [ 打开我的直播间 ]  [ 结束直播 ]   │
└─────────────────────────────────────┘
```

P1 **不强制** App 内摄像头推流；优先把 OBS 路径做顺（与现有 publish API 一致）。

### 4.4 You（我的，对齐 Library/You）

分组列表（ListTile 风格，接近 YouTube You 页）：

1. 资料卡（头像、昵称、邮箱、编辑）  
2. 钱包（余额、充值、流水）  
3. 我的关注  
4. 观看/开播历史*（P1 可桩）  
5. 设置：会话、登出全部  
6. 隐私与安全：导出数据、删除账号、条款链接  
7. 版本号 / 环境  

---

## 5. 视觉与设计 Token

### 5.1 主题

| Token | 建议值（可调） | 用途 |
|---|---|---|
| `bg.app` | `#0F0F0F` | 全局深色（观影） |
| `bg.elevated` | `#212121` | 卡片、底栏、sheet |
| `bg.player` | `#000000` | 播放器 |
| `text.primary` | `#F1F1F1` | 标题 |
| `text.secondary` | `#AAAAAA` | 副文案 |
| `accent` | `#FF0033` 或品牌红 | LIVE、主 CTA（避免直接抄 YouTube 红作为商标暗示时可改品牌色） |
| `accent.soft` | accent @ 15% | chip 背景 |
| `success` | `#3DDC97` | 关注成功等 |
| `danger` | `#FF4D4F` | 错误、删除 |
| `radius.card` | 12 | 封面圆角 |
| `radius.pill` | 999 | chip / LIVE 徽章 |
| `space.xs–xl` | 4 / 8 / 12 / 16 / 24 | 间距阶梯 |
| `font.title` | 16–18 semibold | 卡片标题 |
| `font.body` | 14 regular | 聊天 |
| `font.meta` | 12 | 次要信息 |

**亮色模式：** P1 可只做 Dark（YouTube 观影默认深色）；设置里预留 Light 开关即可。

### 5.2 组件库（建议抽 `apps/mobile/lib/ui/`）

| 组件 | 用途 |
|---|---|
| `AppScaffold` | 统一 bg、状态栏 |
| `AnyBottomNav` | 4/5 Tab |
| `LiveBadge` | LIVE 红标 |
| `LiveCard` | 封面+标题+主播行 |
| `ChannelRow` | 头像+名+Follow |
| `PlayerStage` | 播放器容器 + 结束遮罩 |
| `ChatList` / `ChatComposer` | 聊天 |
| `GiftDock` | 礼物横滑 + 余额 |
| `PrimaryButton` / `GhostButton` | CTA |
| `EmptyState` | 无直播 / 未关注 |
| `MoreSheet` | 底部 ⋯ 菜单 |

### 5.3 动效（克制）

| 场景 | 动效 |
|---|---|
| 进房 | 页 push + player fade-in |
| 送礼 | 底部轻量 overlay 文案/emoji 上浮 1s（非全屏特效编排器） |
| 点赞* | 双击出短暂心形（可选，P1 可砍） |
| 下拉刷新 | Material 标准 |

---

## 6. 关键用户流程

### 6.1 观众 10 分钟路径（对齐 mvp-scope）

```
冷启动 → 会话恢复?
  ├─ 无 → Login(OTP+年龄+条款) → MainShell.Home
  └─ 有 → MainShell.Home
→ 浏览 LiveCard → 点击进房
→ 自动 play HLS
→ 发言 / 拉聊天
→ 打开礼物 → 余额不足则跳钱包沙箱充值 → 送礼
→ 关注主播
→ 结束态或返回
```

### 6.2 主播 Go Live

```
MainShell → Go Live
→ 输入标题 → 开始直播(create+start)
→ 展示 publish 信息 → 复制到 OBS
→ 打开我的直播间预览聊天/礼物
→ 结束直播(stop)
```

### 6.3 合规与安全

| 入口 | 位置 |
|---|---|
| 隐私/条款 | Login 底部 + You → 法律 |
| 年龄确认 | Login 勾选 + Profile |
| 举报 | Room ⋯ 菜单 |
| 导出/删号 | You → 隐私与安全 |

---

## 7. 与现状代码的落差（实施映射）

### 7.1 现状（`apps/mobile/lib`）

| 已有 | 问题（相对本设计） |
|---|---|
| `HomePage` 偏启动台 | 非底 Tab 壳；能力入口按钮化 |
| `FeedPage` Hot/Following | 有基础，缺 YouTube 卡片视觉 |
| `RoomPage` 控制面感强 | 播放器未成为视觉主；礼物/聊天线需重排 |
| `RoomListPage` / Go live | 有，应并入创作入口 |
| `ProfilePage` / `WalletPage` | 有，应挂到 You Tab |
| `StreamPreview` | 保留，装进 `PlayerStage` |
| 无统一 `ui/` Token | 风格易分裂 |

### 7.2 建议目录演进

```
apps/mobile/lib/
  app.dart                 # MaterialApp + 主题
  main.dart
  config/
  navigation/
    main_shell.dart        # BottomNav 容器（新增）
    app_routes.dart
  theme/
    any_colors.dart
    any_text.dart
    any_theme.dart
  ui/                      # 无业务纯组件
    live_card.dart
    live_badge.dart
    ...
  features/
    auth/
    home/                  # HomeTab 发现
    following/             # 可从 feed 拆出
    live/                  # RoomWatch（原 rooms/room_page 重命名或包装）
    go_live/
    you/                   # 聚合 profile+wallet 入口
    search/
  player/
  api/
  realtime/
```

### 7.3 与并行任务板对齐（FL）

| 设计模块 | 任务 ID（见 parallel-tracks） | 优先级 |
|---|---|---|
| MainShell + 主题 Token | FL 基建（建议新增 FL-0） | P0 |
| Home LiveCard 流 | FL-3 | P0 |
| RoomWatch 布局重排 | FL-5 · FL-6 | P0 |
| Go Live 页 | FL-4 | P0 |
| You 聚合 | FL-8 · FL-9 | P0 |
| Search | FL-3 附属 | P1 |
| 送礼动效/点赞 | FL-5 增强 | P1 |
| 连麦 PK UI | FL-X | **禁止 P1** |

---

## 8. 状态与错误体验（YouTube 级「稳」）

| 状态 | UI |
|---|---|
| 列表 loading | 卡片骨架屏（灰块闪烁），非整页只有转圈 |
| 列表空 | EmptyState + CTA「刷新」 |
| 播放失败 | Player 内错误文案 + 重试；保留聊天 |
| 网络断开 | 顶 banner「连接中断」；恢复后自动刷新消息 |
| 403 禁言 | Composer 禁用 + snackbar 原因 |
| 403 封禁 | 回登录或 You 提示 |
| 房间 not live | 结束遮罩，不假装在播 |
| 送礼失败 | 礼物按钮恢复；错误 toast；余额不变 |

---

## 9. 无障碍与国际化（海外优先）

| 项 | P1 |
|---|---|
| 语义标签 | 按钮 `Semantics` / tooltip |
| 字号 | 跟随系统缩放不截断主 CTA |
| 对比度 | 深色底 + 浅字达标 |
| 文案语言 | 默认 **English** UI 字符串；结构支持 ARB 后续 |
| RTL | P1 不阻塞；避免写死 left 动画依赖 |

---

## 10. 埋点（体验验证，可选）

与 `event-dictionary` 对齐，房间路径建议：

| 事件 | 时机 |
|---|---|
| `home.view` | 打开 HomeTab |
| `home.card_click` | 点击 LiveCard |
| `room.view` | 进房 |
| `room.chat_send` | 发言成功 |
| `gift.tap` / 成功 | 送礼 |
| `golive.start` | 开播成功 |
| `auth.login` | 登录成功 |

P1 以验证漏斗为主，不做复杂分析后台。

---

## 11. 分阶段交付（仅 Flutter UX）

### Phase UX-1（P1 dogfood 体验底线）— 建议先做

1. `MainShell` 四 Tab：Home / Following / Go Live / You  
2. `AnyTheme` 深色 Token  
3. `LiveCard` 列表替换按钮墙  
4. `RoomWatch`：上播放器、中信息、下聊天+礼物  
5. 结束态 / 禁言 / 举报入口  
6. Go Live OBS 信息页  

### Phase UX-2（体验加分）

1. 骨架屏、下拉刷新、空态插画  
2. 搜索页  
3. 礼物轻动效、关注动画  
4. 分享 sheet  
5. Player 手势（显示/隐藏控制条）  

### Phase UX-3（后置）

1. 个性化 Home  
2. 通知 Inbox  
3. App 内摄像头推流  
4. 连麦/PK（仅 experimental 开关，非默认）  

---

## 12. 验收清单（设计走查）

### 视觉

- [ ] 全局深色一致，无「一页亮一页暗」  
- [ ] LIVE 徽章与 YouTube 心智一致（红标+大写 LIVE）  
- [ ] 卡片 16:9，标题最多 2 行  
- [ ] 底 Tab 选中态清晰  

### 流程

- [ ] 未登录可浏览 Home* 或强制登录策略产品二选一（建议 P1：**强制登录后主壳**，降低游客态工期）  
- [ ] 登录含年龄+条款  
- [ ] 进房 ≤2 点  
- [ ] 送礼不离开房间页  
- [ ] 主播可复制推流信息  

### 业务边界

- [ ] 无 Shorts 入口  
- [ ] 无 PK/连麦主路径入口（或开关关闭时不可见）  
- [ ] 无电商  

\* 若允许游客浏览，需只读 API 与登录墙策略，另开小节。

---

## 13. 开放问题（需产品拍板）

| # | 问题 | 建议默认 |
|---|---|---|
| Q1 | 游客是否可看播？ | P1 强制登录，减分支 |
| Q2 | 品牌主色是否避开「YouTube 红」？ | 可用自有粉红/品红，LIVE 仍可用高对比红 |
| Q3 | Go Live 用中央 Tab 还是 You 内入口？ | 中央 Tab 更接近创作心智 |
| Q4 | 聊天默认 WS 还是轮询？ | 有 `CENTRIFUGO_WS` 用 WS，否则轮询，UI 不暴露实现 |
| Q5 | 封面图从哪来？ | P1 纯色+标题占位；有 MinIO 头像后再用主播头像铺底 |

---

## 14. 相关文档

- MVP 范围：[mvp-scope.md](./mvp-scope.md)  
- Flutter 并行任务：[p1-parallel-tracks.md](./p1-parallel-tracks.md)  
- 实现状态：[p1-status.md](./p1-status.md)  
- 埋点字典：[event-dictionary.md](./event-dictionary.md)  
- 本地开播：[../runbooks/go-live-local.md](../runbooks/go-live-local.md)  

---

## 15. 修订记录

| 日期 | 变更 |
|---|---|
| 2026-07-23 | 初稿：YouTube 风格 IA/线框/Token/分阶段与代码落差 |

---

*本设计为体验与信息架构规范，不授权使用 YouTube 商标、图标或品牌资源。实现时使用 AnyLive 自有视觉资产。*
