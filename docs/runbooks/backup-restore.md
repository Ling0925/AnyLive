# 生产备份与恢复演练（P2 出口）

> 控制面：Postgres（users/rooms/wallet/pay/…）。媒体与 IM 不在本库。

## 1. 备份对象

| 组件 | 内容 | 工具建议 |
|---|---|---|
| Postgres | 全库（含迁移版本） | `pg_dump` / 托管快照 |
| 对象存储 | 封面/静态资源（若用 MinIO/S3） | 桶版本 / 跨区复制 |
| 密钥 | JWT/OTP/SRS/Pay webhook secrets | 密钥管理器；**不入 git** |
| 配置 | `.env` 生产清单、feature flags | 加密配置仓 / 密钥管理器 |

## 2. RPO / RTO 目标（dogfood → soft launch）

| 阶段 | RPO | RTO |
|---|---|---|
| P1 dogfood | ≤ 24h | ≤ 4h |
| P2 soft launch | ≤ 1h | ≤ 1h |
| P5 GA | ≤ 15 min（连续归档） | ≤ 30 min |

## 3. 演练步骤（至少每季度 1 次）

1. **冻结写流量窗口**（或在 stage 副本上执行，推荐）。
2. 从最近备份恢复到 **隔离** Postgres 实例。
3. 校验：
   - `SELECT version FROM _sqlx_migrations`（或等价）与生产一致；
   - `GET /api/v1/admin/wallet/reconcile` → `balanced=true`；
   - 抽样 `GET /api/v1/me`、房间列表、礼物目录。
4. 记录：备份时间戳、恢复耗时、差异清单、操作人。
5. 归档演练记录：`reports/backup-restore-YYYYMMDD.md`。

## 4. 演练记录模板

```markdown
# Backup restore drill — YYYY-MM-DD
- Environment: stage | prod-shadow
- Backup source / snapshot id:
- RPO achieved:
- RTO (minutes):
- Reconcile balanced: yes/no
- Issues:
- Sign-off:
```

## 5. 与本仓库的衔接

- 钱包对账：`GET /api/v1/admin/wallet/reconcile`
- Stage 发布：`docs/runbooks/go-live-stage.md`
- 本地栈：`./scripts/deploy-test.sh`（**不替代**生产备份演练）

_状态：手册已就绪；勾选「生产备份与恢复演练 1 次」需人工执行并归档报告。_
