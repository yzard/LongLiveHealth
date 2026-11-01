# 独立 OCR 服务

backend_api 接收手机照片/PDF，保存原件并逐页调用 backend_ocr。backend_ocr 使用 Qwen3.8 27B NVFP4 + NInfer 返回结构化化验项目，API 先归档完整服务回复，再校验候选并写 SQLite。原件、审核、趋势和导出都归 API，OCR 不挂载其 data 或数据库。

## 固定版本与硬件

| 项目 | 当前值 |
| --- | --- |
| 引擎 | [NInfer](https://github.com/Neroued/ninfer)，commit `9e163eee4b8acec21ab0ac765107b6a3f287b217` |
| 模型 | [Qwen3.8-27B-nvfp4-NInfer](https://huggingface.co/neroued/Qwen3.8-27B-nvfp4-NInfer)，revision `f0b43ad436b9fa8142c6ed6647c470a6fe409484` |
| 模型文件 | `qwen3_8_27b_nvfp4.ninfer`，23,719,715,844 字节 |
| SHA256 | `74d2c57145e6ff11d1d2faa79594477f9bc903a611af1fb20218189fbbb77d82` |
| 容器 / GPU | CUDA 13.1.2；RTX 5090；需要 NVIDIA Container Toolkit |

模型在镜像构建时下载、校验并放入 `/models/`，不是启动时下载。源码与模型许可证随镜像保留。默认 FP8 KV、单并发、32,768 上下文、8,192 最大输出 token、MTP 推测解码和 thinking；参数见 `src/backend_ocr/config.toml`。当前 GPU 合成验收通过不代表其他硬件可用或真实报告准确率已通过。

## 配置与运行

两个服务分别要求 `--data-dir ABSOLUTE_DIRECTORY`，只加载其根内固定的 `config.toml`；TOML 不包含 data_dir。相对模型路径以该根解析。API `[ocr]` 指定 URL、内嵌 api_key 和 660 秒请求超时。OCR `[server]` 指定监听、内嵌同值 api_key、600 秒整体推理截止时间和限额，`[engine]` 指定引擎及模型文件、生成参数。配置变更重启生效。手机用户 token 不传入 OCR。

`./run_playground.sh` 构建两个镜像，以前台运行 API、OCR 和 Caddy。仅在缺失时生成 playground/backend_api 与 playground/backend_ocr 内的 config.toml（0600），OCR 密钥直接写入两份 TOML，已有配置与稳定密钥不覆盖。旧 api_key_file 配置由离线初始化脚本一次性转换；运行时拒绝该旧字段且不读取密钥文件。API 根读写挂载，OCR 根只读挂载，OCR 无法读取 API SQLite 或原件。独立部署用 LONGLIVEHEALTH_API_DATA_DIR 和 LONGLIVEHEALTH_OCR_DATA_DIR 选择两个根目录。

Compose 的 OCR 只加入 internal inference 网络，没有宿主机端口。8000 是服务入口，NInfer 仅监听容器回环 8002；不直接暴露原生 chat 接口。API 通过 Caddy HTTPS 为手机提供服务，OCR 不通过 Caddy。服务使用 PUID/PGID 降权运行，默认为 1000:1000。

健康检查 GET `/health` 返回 `status=ok` 和 `engine_state`（unloaded/loading/ready/busy/stopping），不加载模型或重置空闲时间。首次提取惰性启动 NInfer，默认空闲 300 秒后回收进程及显存。队列最多 4 个请求（包含正在执行的请求），预处理和推理串行。超时、客户端取消或服务退出会终止并等待 NInfer 进程组，必要时 SIGKILL；释放执行槽后才能处理下一项。服务访问日志只含方法、已知路由、状态和耗时。

## 内部契约

POST `/api/v1/documents/extract`，`Authorization: Bearer <service-key>`，JSON 请求：

```json
{"page":1,"image_url":"data:image/png;base64,...","text_layer":null}
```

仅接受内联 PNG/JPEG。禁止远程 URL、文件路径、任意模型名、messages 和推理参数。请求正文默认最多 64 MiB、解码图像最多 40 百万像素；仅推理副本做 EXIF 转正和缩小至 4,194,304 像素，API 原件不改变。PDF 由 API 渲染逐页 PNG，同时 pdftotext -bbox-layout 提取文本及归一化词坐标；HEIC 用已归档 JPEG 处理副本。内部请求必须显式携带 text_layer：普通图片为 null，PDF 为 {status,text,words}（每词 text/bounding_box）。模型提示只附该层状态和文字，完整词位置与原始 XML 保存在 API。

文本提取 30 秒，原始 XML 上限 2 MiB、可用文字上限 32 KiB、最多 4096 词。available/empty/unavailable/limit_exceeded 状态显式记录；失败/超限不会发送部分文字冒充成功，继续视觉并标记需要核对。文本层是非可信证据，可能只有页眉或与图像冲突，不能当作指令或替代整页视觉。每次 run/page 的证据存 extraction_inputs，通过鉴权接口读取并随导出/删除生命周期管理。

成功回复包含 `model`、`engine=ninfer`、`prompt_version=laboratory-page-v5`、`content`、`raw_response_body`、`observations` 和 `warnings`。坐标为 null 或归一化 [left,top,right,bottom] 数组，与 API 一致。每项保留原名、原结果（包括比较符/文本）、单位、参考范围、异常标记、采样日期、页码和原文引文，`metric_id` 必须为 null。API 二次校验后保存为 pending，人工复核才可确认和映射指标。有效结构中的 warnings 保留完整证据，并使该页 needs_review；不会把已经有效提取的候选判成执行失败。真正的 HTTP、结构或字段错误仍明确失败。

完整原生回复含模型正文、reasoning 和 usage 等实际返回字段。原生响应上限 3 MiB，API 服务回复上限 8 MiB；超过上限显式失败，不截断后冒充完整。结构不合法、页码错误或 `finish_reason` 非 stop 返回 422 `invalid_structured_output`，同时附原始正文及完整原生回复供 API 归档。鉴权 401、请求超限 413、队列满 429、引擎不可用 503、整体超时 504 都是明确失败。

API 将已接收完整 HTTP 回复保存到 extraction_outputs，含失败回复；归档后才检查状态与结构。无回复的网络失败不伪造模型正文。HTTP 调用不在 OCR 客户端内部自动重试；API 持久任务负责有限尝试、租约和手动重试，晚到结果在提交前核对归属及删除状态。

## 验证

```bash
uv sync --locked --all-groups --project src/backend_ocr
src/backend_ocr/.venv/bin/python -m unittest discover -s tests/backend_ocr -p '*.py'
./build_docker.sh
src/backend_ocr/.venv/bin/python tests/docker/deployment.py --ocr --load
```

Docker 构建关口运行 Rust fmt/Clippy/75 项测试、Python isort/Black/10 项 OCR 测试、13 项开发脚本测试及 11 项前端测试。最后一个命令以隔离配置、随机端口、容器和临时卷完成真实 GPU 合成图片、文字 PDF、两页扫描/混合 PDF 的多单位识别及 HTTPS 数据闭环；--load 同时核验 Health 大载荷准入、原始重放、超限及响应性，结束清理自己的资源。未提供任何真实健康报告输入，不能据此标记临床准确性或 iPhone 真机验收完成。

所有 background service 配置凭据直接内嵌到 config.toml，禁止凭据文件与环境/CLI 密钥覆盖。示例与验收见[内嵌凭据审查](../reviews/2026-10-03-embedded-credentials.md)。模板的空 OCR 密钥必须配置后才可启动 OCR；Playground 初始化会生成匹配的有效密钥。OCR 配置对象隐藏密钥表示，非法配置错误不输出原值。

OCR 是 API 的必要服务：[ocr] 没有 enabled，所有上传在归档事务内直接排队，worker 始终尝试提取；原件和人工复核在 OCR 临时故障时继续可用。有效 URL 与内嵌密钥必须在启动时校验，不能通过开关跳过。详见[必要 OCR 服务审查](../reviews/2026-10-03-required-ocr.md)。

## 2026-10-30 Function Health 采集日期核查

历史 `laboratory-page-v3` 提取在第 3、4、6、9 页遗漏 47 个 sampled_at；原始 PDF 右侧有日期，部分旧 source.quote 也包含日期。用当前 v4 重跑这四页，47 项日期均恢复。v5 进一步要求按完整视觉行关联右侧日期，不依赖 PDF 文本层顺序；返回前复核空日期与引文，并明确警告真正没有日期的项目，禁止借用生日、报告生成日期或相邻项目日期。

真实 11 页报告隔离复测共 126 项，全部日期匹配页面。原件和测试输出仅存于忽略的 build/ocr/function-date-audit/，不提交个人健康数据。测试不覆盖生产已确认结果；旧提取不会因为更新提示词而自动改写。

可重复的合成 GPU 回归检查（需要运行中的 OCR 服务及其配置目录）：

```bash
python tests/docker/collection_dates.py --url http://127.0.0.1:8000 --data-dir /data
```

样本含两个不同采集日期、生日、报告生成日期及一个真正无日期的项目；必须正确区分前两项、保留第三项空日期并给出警告。服务凭据只从配置读取，不通过命令行传递。

## Report rescan

Health Report shows View Original, Add Result, Rescan and Delete in one action row on both mobile platforms. Rescan asks for confirmation because it replaces confirmed results, pending drafts and manual edits. The original file and report context remain. New extraction results are pending and require Confirm All before appearing in charts.

`POST /api/v1/reports/rescan` accepts `report_id` and `expected_revision`, authenticates the owner and rejects queued/running extraction or stale revisions with 409. One database transaction clears old results and extraction evidence, increments the report revision, invalidates derived analyses/exports and queues a new extraction job. Failed rescans keep the original file and can use the normal failed-job retry; they do not restore the overwritten results. Other reports and Apple Health records are unaffected. The mobile page follows the new job identity so it starts polling even when opened before a rescan.

Rescan release verification (2026-10-30): API image `report-rescan-20261030` deployed on NAS; Android Build 15 installed and packaged at `/longlivehealth.apk`; iOS Build 69 signed and installed. Rust checks passed with 199 tests and 2 pre-existing ignored tests. Both dedicated mobile action-row/cancel checks passed; no real report was rescanned during acceptance. The longer iOS `testLargeReportCountAndSourceEditing` passed the new action-row checks, report editing/confirmation and long scrolling, but failed at its final marker-editor tap (Cancel was absent); that broader editor check is not counted as passed. APK SHA256: `be5a2e91052d4d90b41f0d8af2979446da5f42a85955daa9f78ed2f7d2480c64`.
