# RC1 无外部依赖验收包

更新时间：2026-07-04

本文是 RC1 无外部依赖验收的入口。它集中引用 `docs/RC1双端QA总索引.md`、桌面 / Android 页面级 QA、`commercial:ci` 最新输出、PostgreSQL disposable 证据和所有 blocked artifact。

本验收包不放宽能力边界，不替代 `docs/当前真实能力边界说明.md`，也不把 Android 证据等同于 iOS 证据。

## 1. 验收结论

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| RC1 无外部依赖验收包 | READY_WITH_BLOCKED_ITEMS | 本机可验证项已汇总；Android 真实 OS 断网拨测已补，Windows 桌面端真实 OS 断网因 loopback / firewall 权限继续 blocked；iOS、生产支付、生产 C2PA/TSA、L3 可售 SLA 和生产 PostgreSQL 切换继续 blocked。 |
| Release owner 评审请求 | SUBMITTED_FOR_REVIEW | `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.json`；人工版 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.md`。 |
| Release owner 决策 | CONDITIONAL_GO_FOR_REVIEW / FINAL_SIGNOFF_NO_GO | `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.json`；人工版 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.md`。RC1 包可进入评审；Windows 桌面端窗口执行仍缺提权 / 非 loopback 环境，最终签字继续 NO-GO。 |
| 主索引 | READY | `docs/RC1双端QA总索引.md` |
| 机器摘要 | READY | `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.json` |
| 人工摘要 | READY | `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.md` |

## 2. 自动化门禁

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| `npm run commercial:ci` | READY | 2026-07-10 完整复跑通过，最终输出 `HiddenShield commercial CI OK`；本轮包含桌面云同步 `eventResults` 消费修复，`conflict_payload_changed` / `rejected_invalid_event` 不再误清队列。 |
| `vault_records.file_type` 聚合门禁 | READY | `commercial:ci` 已包含 `Vault file_type backfill contract`，输出 `vault:file-type-backfill-contract OK`；单独复核 `npm run vault:file-type-backfill-contract` 通过。 |
| 双端一致性合同 | READY | `npm run dual:contract` 通过。 |
| 空白检查 | READY | `git diff --check` 未发现空白错误，仅有 Windows 换行提示。 |

## 3. 运行态 QA

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| 桌面安装版 Batch 2 | READY | `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json`，sanity 截图 `tmp-ui-qa/desktop-batch2-qa/97-desktop-batch2-current-sanity.png`。 |
| Android Batch 2 | READY | `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json`，截图目录 `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/screenshots/`。 |
| 云同步专项 | READY | 桌面 `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783067038401.json`；Android `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783067038401.json`；强制 ready `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json`；2026-07-10 `npm run cloud:sync-reliability-contract` 已补桌面 `eventResults` 消费合同。 |
| 后端 / 桌面 event disposition | READY | 后端运行态证据 `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783067038401.json`；桌面回归 `desktop_flush_event_results_keep_conflicts_failed` 确认 `accepted` / `duplicate` 才清队列，冲突 / 拒绝保持 failed 诊断。 |
| Android 真实 OS 断网拨测 | READY | `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-os-network-disconnect-drill-20260704.json`；断网截图 `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-network-off-20260704.png`；恢复截图 `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-network-restored-20260704.png`。 |

## 4. PostgreSQL Disposable 证据

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| Migration smoke | READY | `tmp-ui-qa/postgres-migration/postgres-migrate-smoke-1783021160601.json` |
| Runtime aggregate | READY | `tmp-ui-qa/postgres-runtime-aggregate/cloud-postgres-runtime-qa-1783053449984.json` |
| SQLite -> Postgres import smoke | READY | `tmp-ui-qa/postgres-import/postgres-import-smoke-1783053193204.json` |

这些证据只证明 disposable PostgreSQL 下 auth / sync / registry / migration / import smoke 可本机验证，不代表生产云版权库已经切 PostgreSQL。

## 5. Blocked Artifact

| 阻断项 | 状态 | 证据 | 放行条件 |
| --- | --- | --- | --- |
| iOS 页面级 / 公开权利 V3 QA | BLOCKED | `tmp-ui-qa/rc1-no-external-acceptance/20260704/ios-qa-blocked-20260704.json`；官方 runner 产物 `tmp-ui-qa/ios-public-rights-v3-runtime/1783114433471/ios-public-rights-v3-runtime-qa-1783114433471.json` | macOS + Xcode + iOS Simulator 或真机，复跑 `npm run rights:ios-public-rights-v3-runtime-qa` 并产出 passed artifact。 |
| Windows 桌面端真实 OS 断网拨测 | EXECUTION_BLOCKED_MISSING_ELEVATED_OR_NON_LOOPBACK_ENVIRONMENT | `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/windows-desktop-os-network-drill-execution-20260704.json`；安排清单 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill-schedule-20260704.json`；聚合记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/os-network-disconnect-drill-record-20260704.json` | 2026-07-04 20:30 窗口已执行环境判定：当前会话不是管理员，安装版仍走 `127.0.0.1:43188`，且未提供 LAN / staging backend；不能真实切断桌面 app-backend 路径。 |
| 公开权利 completion | BLOCKED | `tmp-ui-qa/public-rights-completion/public-rights-completion-gate-1782976680658.json` | 生产 C2PA/TSA、iOS QA、外部 npm 发布、release 样本池、客户签字。 |
| L3 production readiness | BLOCKED | `tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1783113551653.json` | 真实告警平台验证、试点客户签字、真实用户 MP4 样本 manifest。 |
| PostgreSQL production readiness | BLOCKED | `tmp-ui-qa/postgres-production-readiness/cloud-postgres-production-readiness-gate-1783053429272.json` | staging 压测、备份恢复、observability、切换 runbook、release owner signoff。 |
| SQLite production shutdown | BLOCKED | `tmp-ui-qa/postgres-sqlite-shutdown/cloud-postgres-sqlite-shutdown-gate-1783053429239.json` | P5 production readiness 通过后再评审。 |

## 6. 隐私边界

本验收包只引用 metadata-only artifact、截图、报告摘要和运行态 JSON。它不要求同步或归档原始媒体、保护副本媒体、本地路径、object ref、签名 URL 或可还原媒体内容。

## 7. 当前推荐下一步

release owner 提供提权 Windows QA operator 或 LAN / staging backend endpoint 后重跑 Windows 桌面端断网拨测；通过后回写 RC1 包并复跑 `cloud:sync-reliability-contract`、`dual:contract` 和 `git diff --check`，否则最终 RC1 签字继续 NO-GO。
