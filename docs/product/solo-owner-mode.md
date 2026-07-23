# Solo owner 运作口径（个人项目）

> **生效：** 2026-07-23 · 仓库 owner 明确决策  
> **适用：** 当前 AnyLive 由个人推进、本地 / 自测 dogfood  
> **不改变：** 完整技术栈与实现目标（API / Flutter / H5 / Admin / 媒体 / 资金沙箱等）

## 1. 要什么

| 要 | 说明 |
|---|---|
| 完整实现 | 登→刷→看→聊→送→管 控制面 + 客户端 UX + 本地可推可看 |
| 可重复打包 | 每阶段可 `deploy-test`、打 APK / H5、跑 dogfood 脚本 |
| 诚实标注 | dev OTP ≠ 真邮件；本地 1k×3min ≠ stage 15min soak；FEATURE_PK/COHOST 默认 OFF |
| 有 bug 就修 | 自测反馈 → 修 → 重部署 / 重打包 |

## 2. 不要什么（流程关闸）

| 不要 | 说明 |
|---|---|
| **签字仪式** | risk-accept 表单、TL/PM 签名、归档 signed 副本 **不是** 本仓库推进条件 |
| **强制设备矩阵 Pass** | 有真机自测即可；不必填满 matrix 模板勾 Pass |
| **强制 Recording URL** | 不必为 V-FL-2 录屏归档才能继续 |
| **缺陷会纪要** | 无多人团队时不做形式会议 |
| **OBS「满 7 天」才算过** | 需要时自己推流验证即可，不卡日历 KPI |
| **用流程挡实现** | 不以未签字阻止修 bug、做 UX、打包自测 |

`docs/runbooks/*-risk-accept.md` 等表单 **仅作可选模板**（若以后多人协作可再用），**solo 下不要求填写**。

## 3. Wave2 / 看板怎么读

[p1-parallel-tracks](./p1-parallel-tracks.md) 中 V-* 在 solo 下：

| ID | Solo 口径 |
|---|---|
| V-BE-1 | **waived (solo)** — 本地/测试栈可用 dev OTP；**不**声称真 ESP |
| V-BE-2 | **waived (solo)** — 以本地 1k 基线为参考；**不**声称 stage 15min soak |
| V-AD-1 | **optional** — Admin 自己点通即可，无 footer 签字关闸 |
| V-FL-1 / V-FL-2 | **optional** — 真机/H5 自测即可，无 Pass 勾 / Recording URL 关闸 |
| V-ALL-1 | **n/a (solo)** — 无多人缺陷会 |
| V-ALL-2 | 仍建议主题 commit（工程习惯，非签字） |

自动化 `DOGFOOD_*_PASS` = 控制面绿，便于回归；**不是**合规/上架签字。

## 4. Agent / 协作者纪律

1. **禁止**催促 owner 填签名表、伪造 signed 归档。  
2. **禁止**把「未签字」当成阻塞实现或打包的理由。  
3. **禁止**把 waived 写成「真 OTP 已接」或「15min soak 已过」。  
4. 默认动作：部署栈、跑 smoke、修功能/UX、重打客户端；自测由 owner 做。  
5. 若以后组队或对外放量，再恢复多人或上架所需流程（ESP、stage soak、商店等）。

## 5. 相关

- 打包自测清单（去签字关闸后）：[wave2-self-test.md](../runbooks/wave2-self-test.md)  
- 实现状态：[p1-status.md](./p1-status.md)  
- 阶段规划仍以 [06](./06-P1进度评审与后续规划.md) 为技术范围参考；**流程条款以本文为准覆盖 solo 场景**
