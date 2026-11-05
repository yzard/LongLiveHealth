# API 内置血检 OCR 与共享推理

Long Live Health 仅部署 backend_api 一个业务服务。上传的 PDF/图片仍先归档，持久化后台任务负责逐页渲染、文字证据、图像缩放、提示、模型请求、结构验证与草稿保存。用户 Confirm All 才将指标写入正式结果。没有独立 OCR HTTP 接口或 Python/GPU 容器。

## 一份配置

API 使用 `--data-dir /data`，只读取 `/data/config.toml`。`[llm]` 包含共享 `/v1` URL、模型、内嵌 API 密钥、超时和生成参数；OCR 与 Analytics 使用同一个配置。`[extraction]` 只指定 OCR 输出 token 数和图像像素预算，没有第二套地址或密钥。`analysis_enabled` 控制可选的分析功能，OCR 始终可用。

Desk 实际模型配置为 `qwen3.8-flash-next-iq3_s`，API 为 `http://192.168.1.20:8080/v1`。其服务位于 `/home/zyin/dev/local_llm`，内部 Strata 端口 8081 不提供给客户端。模型加载、空闲卸载和显存管理全由 local_llm 负责；Long Live Health 不打包模型或启动模型进程。

当前共享上下文 16,384 tokens；OCR 输出预算 8,192、图像像素预算 2,097,152，Analytics 输出预算 4,096，关闭 thinking 以输出严格 JSON。凭据在 API operator TOML 中内嵌，0600；不放入手机、镜像、环境变量、CLI 参数或单独密钥文件。模板只有空占位，不会自动制造无效的共享服务密钥。

## 调用与并发

Rust OCR 模块复用 API 的 HTTP provider，发送 OpenAI 风格的文字与 image_url。提示词位于 `src/backend_api/prompts/laboratory.txt`，保留每个视觉行的采集日期、原始结果、原单位、参考区间和页码引文。原件和 PDF 文字层都属于不可信证据，不作为指令。

OCR 与 Analytics 在 API 内共用一个推理槽位，避免同时向大模型发出多项生成。普通浏览、上传、审核和同步不占用这个槽位。OCR 请求中的 CPU 图片处理进入已有后台 CPU 执行器，任务继续使用租约续期和取消机制；不因手机锁屏而丢失服务器任务。

模型的完整 raw_response_body、content、model、engine=strata 和 prompt_version 随 run/page 归档。截断回复、无效 JSON、错误页码、猜测 metric_id 均失败并保留原始证据。缺失名称、结果或采集日期沿用现有逐项诊断，不能静默进入 Browse。没有响应或超过响应限额时不伪造原始输出。

## 构建与验证

`./build_docker.sh TAG` 构建 Android APK 和单个 backend_api 镜像，并执行相关检查。Desk 资源有限，构建前先 `systemctl --user stop local-llm.service`；构建完成后再启动服务并执行真实推理验证，不让编译与模型争用内存。

Docker 构建执行 Rust fmt、Clippy、单元和集成测试。`probe-providers` 使用唯一的共享配置发送合成文字探针。`tests/docker/unified_inference.py` 只针对带 `.synthetic-check` 标记的隔离 API 数据根，验证合成图片上传、采集日期、pending/confirm、真实 Analytics 及完整模型归档，不触碰生产库。

## 2026-10-30 Function Health 采集日期核查

历史 `laboratory-page-v3` 提取在第 3、4、6、9 页遗漏 47 个 sampled_at；原始 PDF 右侧有日期，部分旧 source.quote 也包含日期。用当前 v4 重跑这四页，47 项日期均恢复。v5 进一步要求按完整视觉行关联右侧日期，不依赖 PDF 文本层顺序；返回前复核空日期与引文，并明确警告真正没有日期的项目，禁止借用生日、报告生成日期或相邻项目日期。

真实 11 页报告隔离复测共 126 项，全部日期匹配页面。原件和测试输出仅存于忽略的 build/ocr/function-date-audit/，不提交个人健康数据。测试不覆盖生产已确认结果；旧提取不会因为更新提示词而自动改写。

可重复的共享模型日期回归检查（使用 API 配置目录，需安装 Pillow）：

```bash
python tests/docker/collection_dates.py --data-dir /path/to/api-data
```

样本含两个不同采集日期、生日、报告生成日期及一个真正无日期的项目；必须正确区分前两项、保留第三项空日期并给出警告。服务凭据只从配置读取，不通过命令行传递。

## Report rescan

Health Report shows View Original, Add Result, Rescan and Delete in one action row on both mobile platforms. Rescan asks for confirmation because it replaces confirmed results, pending drafts and manual edits. The original file and report context remain. New extraction results are pending and require Confirm All before appearing in charts.

`POST /api/v1/reports/rescan` accepts `report_id` and `expected_revision`, authenticates the owner and rejects queued/running extraction or stale revisions with 409. One database transaction clears old results and extraction evidence, increments the report revision, invalidates derived analyses/exports and queues a new extraction job. Failed rescans keep the original file and can use the normal failed-job retry; they do not restore the overwritten results. Other reports and Apple Health records are unaffected. The mobile page follows the new job identity so it starts polling even when opened before a rescan.

Rescan release verification (2026-10-30): API image `report-rescan-20261030` deployed on NAS; Android Build 15 installed and packaged at `/longlivehealth.apk`; iOS Build 69 signed and installed. Rust checks passed with 199 tests and 2 pre-existing ignored tests. Both dedicated mobile action-row/cancel checks passed; no real report was rescanned during acceptance. The longer iOS `testLargeReportCountAndSourceEditing` passed the new action-row checks, report editing/confirmation and long scrolling, but failed at its final marker-editor tap (Cancel was absent); that broader editor check is not counted as passed. APK SHA256: `be5a2e91052d4d90b41f0d8af2979446da5f42a85955daa9f78ed2f7d2480c64`.
