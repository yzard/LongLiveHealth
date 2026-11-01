# Desk 本地备份核查 — 2026-10-31

核查基线：GitHub `main` 的 `ca5dcf4`。Desk 同步前备份的 manifest 标识为 `20261101T020727Z`（UTC），包含 4 个与最新版不同的文件、7 个旧路径跟踪文件、15 个额外文件。核查时原件保存在 Desk 的私有仓库外备份中；没有将运行数据、构建缓存或备份原件加入 Git。随后按用户要求，已删除下述旧构建方案的 8 个文件及 Mac 核查副本。其他核查材料保留。

## 已存在于最新版的内容

4 个不同文件是 `.gitignore`、`docs/api.md`、血压/历史列表核查文档和完整 Browse 参考区间核查文档。备份版本较旧：最新版已递归忽略 Rust `target`，更新已清理的 API 清单，并补全后续客户端与 232 项指标的核查结果。没有遗漏的 API 实现文件。

14 个 `tests/ios/Core` 文件均在 `tests/shared/Core` 中有对应文件：

| 测试文件 | 核查结果 |
| --- | --- |
| APIClientTests.swift | 与最新版相同 |
| EntryDraftTests.swift | 与最新版相同 |
| EntryMutationTests.swift | 与最新版相同 |
| LaboratoryPresentationTests.swift | 与最新版相同 |
| ReportImportInboxTests.swift | 与最新版相同 |
| SourceNamesTests.swift | 与最新版相同 |
| TrainingFileTests.swift | 与最新版相同 |
| ChartWindowTests.swift | 已改为日历周期、夏令时与历史翻页测试；旧通用缩放接口已移除 |
| HealthSyncBatchTests.swift | 已改为按编码字节数混合类型分块；旧记录数/请求大小上限测试不再适用 |
| HealthSyncPipelineTests.swift | 已适配显式预读策略，并新增大块先上传的覆盖 |
| HealthSyncProgressTests.swift | 保留旧测试，并新增 Recovery HRV 名称映射 |
| JSONValueTests.swift | 保留旧测试，并新增来源标识回归 |
| LocalArchiveTests.swift | 已适配当前断点对象，并新增主线程外编解码测试 |
| OverviewSummaryTests.swift | 保留旧测试，并新增今天/七天/三十天及睡眠目标回归 |

旧批次测试中的转义字符场景有保留价值。它已改写到当前 `HealthSyncBatchTests.swift`：换行、引号和反斜杠必须按 JSON 编码后的字节数触发分块，序列化后重放必须保留所有记录，游标只附在最后一块。没有恢复已移除的上传限制。

## 未恢复的旧构建方案

其余 8 个文件是 `build_cleanup.py`、`build_lifecycle.sh`、`model_layers.py`、对应的 3 个测试文件，以及 2 份构建清理文档。它们属于旧方案，没有被当前构建入口调用：

- 生命周期脚本依赖独立 BuildKit builder、临时工作目录及 `qwen-model` stage；当前入口包含 Android APK 构建，OCR 使用 `model` stage，原测试替身也没有覆盖当前组合构建流程。
- 清理器使用旧的 Xcode 目录清单，不覆盖当前 DeviceDerivedData 等产物；其自动删除依赖缓存的策略并非当前构建行为。文档声称所有入口已接入自动清理，与当前实现不符。
- OCI 验证器只检查标签、描述符大小和层数，没有核验实际 blob 内容哈希，不能据此承诺缓存模型完整性。

没有将这些无调用方的脚本及过期成功声明恢复为现行功能，也没有执行清理、删除缓存或改变构建策略。当前 Dockerfile 中固定模型版本及下载校验仍保留。

验证：共享 Swift Core 测试全部通过（71 项）；同步批次转义/重放回归通过。产品源码和构建入口未变，因此本次不重新部署手机应用或服务。

清理结果：3 个旧构建脚本、3 个配套测试和 2 份过期文档已从 Desk 备份和 Mac 核查副本中删除。当前源码、构建缓存、Docker 镜像、安装包、签名材料和运行数据未改动。
