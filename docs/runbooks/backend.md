# 后端启动与运维

> Current Desk test deployment (2026-10-15): direct Rust HTTP at
> `http://192.168.1.20`, host port 80 mapped directly to API container port 8080.
> Caddy is stopped. `playground/compose.lan.yaml` on Desk now contains the
> source-controlled `docker/docker-compose.direct.yaml` override; its Caddy profile
> prevents ordinary Compose startup from reintroducing the proxy. Data, credentials,
> image and phone origin are unchanged. Historical proxy instructions below describe
> previous deployment states; Caddy is intended for the later NAS deployment.


本版本提供报告原件归档、模型提取、人工复核、趋势、健康原始数据同步、完整导出与删除，以及默认关闭的个人血脂分析。具体字段见 [API 契约](../api.md)，服务行为见 [生命周期审查](../reviews/2026-10-02-server-lifecycle-and-load.md)，当前配置约定见 [必要 OCR 服务审查](../reviews/2026-10-03-required-ocr.md)。

## Playground 启动

从仓库根目录执行 `./run_playground.sh`，需要 Docker/Compose、Python 3、NVIDIA Container Toolkit 和 RTX 5090。OCR 镜像包含约 23.7 GB 的固定模型文件；首次构建需要对应下载和磁盘空间。脚本不接收参数，每次先运行构建检查和镜像构建，再以前台 Compose 启动；持续显示日志，Ctrl+C 停止容器并保留数据。构建失败不会启动或替换服务。

Playground 使用独立 Compose 项目 `longlivehealth-playground`，分别将 `playground/backend_api/` 读写挂载到 API `/data`、`playground/backend_ocr/` 只读挂载到 OCR `/data`，两者都用 `--data-dir /data` 读取自己的 `config.toml`。API 配置的监听地址是容器内的 `0.0.0.0:8080`；API 仍经 Caddy HTTPS 对外，不额外暴露明文后端端口。脚本从解析后的 Compose 配置读取端口并显示地址；LAN IP 不等于 TLS 可用域名。

启动脚本在缺失时创建两个服务根目录内的 `config.toml`（0600），自动生成的同一 OCR 密钥分别内嵌到 API `[ocr].api_key` 和 OCR `[server].api_key`。已有密钥与其他配置保留，不再生成独立密钥文件；API 保存 SQLite、raw、derived、tmp 和 exports，OCR 不访问 API 根目录。脚本导出当前用户的 PUID/PGID。首次 API 启动自动创建 `admin` / `admin`，登录后必须先改密码。管理员也可以在 iOS 设置中添加、删除普通账号，或在终端建号：

```bash
docker compose -p longlivehealth-playground -f docker/docker-compose.yaml exec --user "$(id -u):$(id -g)" backend_api \
  /app/longlivehealth --data-dir /data create-user --username yourname
```

建号时隐藏输入密码，长度 12–1024 字节。自动化可用 `--password-stdin` 从安全输入通道传入，不要把密码写进命令行参数。默认管理员为 admin/admin，首次登录必须改密码。存在旧 playground/config.toml、data/、backend_ocr.toml 或 secrets/ocr-key 时，脚本先停止自己的 Compose 项目，再迁移目录、配置和稳定密钥；旧配置含 api_key_file 或独立 ocr-key 时也先停服，一次性读取旧值并原子写入 TOML，两份配置确认持久化后才删除项目内旧文件；分析密钥同样内嵌。目标冲突、密钥不一致或活跃数据锁会失败，不覆盖。目录整理保留数据库 sidecar 与原件；schema 升级由 API 启动负责。Playground 直接初始化当前 SQLite schema；脚本不会清空宿主机 data 目录，API 只执行明确的 v1 → v2 账号状态升级及 v2 → v3 同步计数索引升级。

宿主机调试同样显式传入绝对数据目录；初始化只在缺失时写入固定的 config.toml，TOML 不接受 data_dir。init-config 写入空密钥占位，必须先在 config.toml 填入与 OCR 相同的 api_key 才能通过校验、建号或启动：

```bash
cd src/backend_api
cargo run --locked -- --data-dir /absolute/path/backend_api init-config
cargo run --locked -- --data-dir /absolute/path/backend_api check-config
cargo run --locked -- --data-dir /absolute/path/backend_api serve
```

原配置文件存在时不会覆盖，配置初始化使用 0600 权限。模型等非凭据相对路径按 TOML 所在目录解析；修改后重启。后端 Cargo.toml、Cargo.lock 和 .cargo/config.toml 均位于 src/backend_api/，直接使用 Cargo 时从该目录执行；根目录不建立 Rust workspace。Rust 中间产物位于 `build/backend_api/target/`，镜像构建将最终二进制暂存于构建阶段的 `dist/backend_api/`。iOS 产物规则见其构建指南。

## 模型服务

backend_api 接收原件并持久化任务；backend_ocr 是独立 FastAPI 服务，管理 Qwen3.8 27B NVFP4 + NInfer 推理。PDF 由 API 渲染为逐页 PNG，HEIC 使用已归档的 JPEG 处理副本；原件均留在 API。服务通信、配置和运行限额见 [OCR 运维](ocr.md)。

- API `[ocr]` 配置 `url`、`api_key` 和 `timeout_seconds`。OCR 是必要服务，容器连接 `http://backend_ocr:8000`，宿主机模板连接配置的本地 OCR 地址；URL、有效密钥和超时均须配置。
- OCR 服务只接受内联 PNG/JPEG，不接受远程 URL、文件路径、任意 messages 或客户端模型配置。当前没有 document_parser 第二阶段。
- `providers.analysis` 使用独立 OpenAI 兼容 `/v1/chat/completions` 接口，个人血脂分析默认关闭。其模型/extra_body 仍由管理员明确配置，凭据直接写入 `[providers.analysis].api_key`；空串表示该本地接口无需鉴权。
- OCR 请求不进行 HTTP 层重复调用；重试由 API 持久任务与租约负责。已确认修订不会被重跑覆盖。分析接口的临时错误最多三次调用，间隔 1、2 秒。

在 `src/backend_api/` 执行 `cargo run --locked -- --data-dir /absolute/path/backend_api probe-providers`：OCR 只检查服务存活，不加载模型；分析发送合成文字。该命令不证明视觉识别正确。OCR 必须配置且 worker 始终运行；上传归档事务直接创建 queued 任务。OCR 暂时不可达时原件仍归档，提取任务显示 failed / extraction_failed，可在服务恢复后重试；不再存在 blocked / provider_disabled 模式。

## 账户与隔离

首次服务启动创建 `admin:admin`，如果 admin 已存在则保留密码。API 启动时 `RESET_ADMIN_PASSWORD=true` 将 admin 密码重置为 `admin`、恢复启用状态、撤销全部会话并清除该账号的登录限流窗口。这个变量仅在 serve 的启动边界读取，必须为 true/false，缺省 false；重置后应移除或设为 false，否则下次重启仍会重置。

默认密码登录返回 `must_change_password=true`。服务端允许该会话读取自身身份、改密码或退出；所有业务接口和管理员接口返回 403/password_change_required。新密码必须为不同的 12–1024 字节值，提交当前密码验证后原子更新，撤销全部会话，用户重新登录。admin 不能删除或禁用；只有 admin 能列出、创建、删除其他账号，删除复用完整数据清理与会话撤销流程。

服务仍可放在现有 Caddy 后面。iOS 接受用户输入的 HTTP 或 HTTPS 地址；HTTPS 正常验证证书，禁止跨地址重定向携带 token。Compose 自带 Caddy 默认配置保持 HTTPS，可用部署 override 配置自己的 HTTP 入口。

```bash
cd src/backend_api
cargo run --locked -- --data-dir /absolute/path/backend_api reset-password --username yourname
cargo run --locked -- --data-dir /absolute/path/backend_api disable-user --username yourname
```

用户名规范化为小写；重置密码和禁用撤销全部旧会话，进行中的旧凭据登录也不能重新建立有效会话。会话默认一天。数据库只保存 token 摘要。禁用保留档案；删除账户由已认证用户显式确认用户名完成。

每个受保护请求由会话确定 user_id，不能由请求覆盖。跨用户资源按 404 处理。日志仅记录服务器生成的请求 ID、方法、路由模板、状态和耗时，不记录原文、文件名、模型响应、密码或 token。响应 `Cache-Control: no-store`。

## 独立部署 Docker / Caddy

```bash
./build_docker.sh
python3 src/development/data_roots.py --project "$PWD"
export PUID="$(id -u)" PGID="$(id -g)"
docker compose -f docker/docker-compose.yaml up -d
docker compose -f docker/docker-compose.yaml exec --user "$PUID:$PGID" backend_api \
  /app/longlivehealth --data-dir /data create-user --username yourname
```

直接 Compose 默认使用相同的两个服务根目录，应与 playground 二选一运行，避免同时打开同一数据库。独立部署可以用 LONGLIVEHEALTH_API_DATA_DIR 和 LONGLIVEHEALTH_OCR_DATA_DIR 指向另外两个绝对目录，先在各目录准备 config.toml，API `[ocr].api_key` 与 OCR `[server].api_key` 必须一致。所有服务配置用的 keys/tokens/passwords 直接内嵌在各自 TOML，不能使用文件引用或环境/CLI 密钥覆盖。Compose 内部后端 HTTP，Caddy 对外 HTTPS。默认 localhost 使用本地 CA；真机需使用受信任的证书和可访问地址。配置 `LONGLIVEHEALTH_DOMAIN` 为自己的域名并安排解析和入口端口。没有替用户发布公共服务。

`docker/config.toml` 是 API 首次初始化模板；运行时配置是 `/data/config.toml`。模型 URL 在容器内解析，127.0.0.1 指容器自身，需改为真实模型服务地址或同网络服务名。PUID/PGID 默认 1000，入口验证 UID/GID/UMASK 并对专用 data 卷降权；管理员 exec 使用对应 UID。不要将无关目录挂载为 `/data`。

正式停止用 `docker compose -f docker/docker-compose.yaml stop`，不要 `down -v` 删除持久卷。测试脚本仅为自己创建的独立测试项目使用 `down -v`。

## 数据、复制与恢复

SQLite schema v3，启用外键、WAL 和 FULL synchronous。为保留已部署的数据，v1 在启动事务中升级到 v2（增加 must_change_password）；v2 在启动事务中增加有效设备记录的部分索引并升级到 v3；空库直接初始化 v3，未知版本拒绝启动。升级前停止服务并备份完整 data 目录。同一 data 只允许一个服务进程；管理员命令可独立运行。

停服并停止管理员写入后，复制整个 data（包含仍存在的 SQLite sidecar）；根内的 config.toml 已含服务凭据。OCR 的独立根目录也应复制，其密钥也只在 TOML 内。恢复到独立目录/卷，用相同版本启动核对账户、原件、修订与导出。当前只支持 schema 7；服务器启动不执行旧版本升级链。没有在线备份或自动备份机制。

报告/账户删除先在事务内使内容不可访问并撤销有关任务或会话，再由持久清理队列删除文件。旧上传 ID 和健康删除事件保留最小抑制标记，避免迟到上传复活。用户自己导出的文件、手机分享目的地和独立备份不在服务器删除范围内。

导出 v1 在一致数据库快照中生成 JSONL、CSV、原件和版本清单，随后再次核对数据版本；数据变化使旧导出不可下载。失败/过期导出可删除后重新生成。CSV 防公式执行，JSONL 保留原字符串；包含健康有效修订原件、OCR/解析完整回复和人工修订。

原件路径统一为 `raw/<user_id>/<source>/<file_id>.<extension>`；Apple Health 使用 `apple_health`，报告使用诊所/实验室名称。目录在首次写入该来源时创建，不预建 `photos`、`documents` 等文件类型目录。HEIC 原件保留，derived 下 JPEG 仅供 OCR。ZIP 按原件索引路径收录文件。

运行期间临时文件由既有维护周期异步清理；HTTP 取消不会提前删除阻塞任务仍在使用的文件。正常停服等待 worker 和已准入 CPU 闭包，再清理并关闭 SQLite；启动在独占 data 锁下恢复 tmp 和没有数据库记录的原件。不要手工向 raw 添加健康报告。PDF 内容解析仍在受大小约束的进程内，公众版需要额外资源隔离验证。

## 验证

```bash
cd src/backend_api
cargo fmt --all --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cd ../..
docker compose -f docker/docker-compose.yaml config --quiet
src/backend_ocr/.venv/bin/python tests/docker/deployment.py --ocr --load
```

最后一项使用 OCR 组件虚拟环境（见 OCR 运维），启动独立合成测试容器，验证实际 GPU 图片/文字 PDF/扫描与混合 PDF 多单位识别、惰性加载与空闲卸载、HTTPS、归档、复核、趋势、导出、重启和删除；--load 追加并发 32 MiB 单 payload /40 MiB 批次、超限原子拒绝和完整重放，记录 API RSS 与状态延迟，再清理自己的临时卷。省略 --ocr 会在上传前停止合成测试的 OCR 容器，核验真实故障下失败任务、原件归档与人工复核；--ocr 才运行真实 GPU 识别。这个测试选项不控制 API 的 OCR 配置或功能。

CPU 或 Health 入口槽满时返回 429，保留相同批次 ID 和内容重试；413 必须缩小请求，不能重试被截断的原始载荷。性能测量写入 build/deployment-check/load-result.json；它是当前机器的短时合成测试结果，不是生产 SLA。当前验证记录见[服务端验收](../reviews/2026-10-02-server-lifecycle-and-load.md)。

### Apple Health sync I/O and writer-lock optimization (2026-10-15)

Desk logs showed full sync batches taking approximately 6 seconds while background
writers exhausted SQLite's 5-second busy timeout. This was reproducible with
1,000 synthetic records carrying approximately 3 KB of original data each on the
same deployment filesystem: 6,226 / 6,399 / 6,378 ms before the change.

Health sync now stages and flushes originals with at most eight concurrent writes
before acquiring `BEGIN IMMEDIATE`. It rechecks connection ownership and batch
replay inside the transaction, publishes only accepted revisions, flushes each
archive directory once, and commits only after the originals are durable. Failed
or canceled staging remains owned by the existing temporary-file cleanup lifecycle;
failed publication rolls back all database changes. Startup orphan recovery still
handles published files from an uncommitted transaction.

For batches without deletions (excluding manual edits), one indexed read retrieves
current versions and tombstones for all incoming keys. Prior payload reads are
bounded by incoming payload lengths. Mixed deletion batches retain ordered processing.
Exports are invalidated once per batch, and already-stale exports are not rewritten.
The record limit remains 1,000; the existing iOS client benefits without an update.

After optimization, the same disk benchmark took approximately 1.0 second per batch.
Instrumented runs held the writer lock for approximately 0.12–0.13 seconds; file
staging occurred outside that lock. These are isolated synthetic measurements, not
an end-to-end iPhone import measurement. `health sync batch committed` logs record
counts, staging, lock wait, writer-lock duration and total duration without payloads
or patient identifiers. Query-plan inspection on the live schema confirmed primary-key
lookups for both current records and tombstones, without a health-record table scan.

Reproduce the isolated benchmark with `TMPDIR` on the deployment filesystem:

```sh
TMPDIR=/path/on/deployment/filesystem cargo test --manifest-path src/backend_api/Cargo.toml --test backend_api sync_disk_benchmark -- --ignored --nocapture
```

Validation: formatter, Clippy with warnings denied, and all backend tests passed
(134 passed, 2 intentionally ignored; the disk benchmark was run separately).
A regression holds a separate SQLite writer lock while staging proceeds, injects
an archive publication failure, checks rollback, and verifies retry and replay.

Deployed on Desk at 2026-10-15 14:33 EDT, image
`sha256:2306f6c9120f881234c32224e087bed63fa60cd5e4f5dddb449a501395975d5f`.
The API reported ready and `http://192.168.1.20/` returned HTTP 200. No new phone
sync request had arrived during this immediate post-deploy check. The previous
image is retained as `longlivehealth-backend-api:before-sync-optimization`.

### Rejected 10,000-record experiment (2026-10-15)

A temporary 10,000-record limit was tested on Desk. With synthetic 3 KB payloads,
the debug disk benchmark measured 9,924 / 11,138 / 10,506 ms, with writer-lock
durations of 976 / 1,426 / 1,063 ms. Throughput was similar to 1,000-record batches.
The user canceled this change after reviewing those results. The server was restored
to the previously verified optimized 1,000-record image, and source/client limits
were restored to 1,000. SQL and writer-lock optimizations remain active. Build 13
was archived and locally exported but never uploaded to App Store Connect; build 12
remains the published client. The unfinished build 13 UI run was canceled.

### Health coverage response interruption (2026-10-15)

After the sync throughput fix, repeated Sync Now attempts stopped at coverage
loading. Caddy logged `aborting with incomplete response` and upstream
`connection reset by peer` for `POST /api/v1/health/coverage`, while API handler
logs recorded HTTP 200. The container had no restart/OOM event, and the preceding
1,000-record sync had committed in 781 ms (856 ms including ingress).

The coverage handler did not consume its POST body. It now uses the bounded Bytes
extractor after authentication, so the body is consumed before constructing the
response. This retains compatibility with both an empty body and `{}`. A real-TCP
regression seeds 300 synthetic coverage rows, sends request headers separately,
asserts that the response waits for the body, then verifies the complete JSON and
five repeated HTTP requests. The body-order assertion failed before the fix and
passed afterward. Initial ordinary HTTP tests alone did not reproduce the proxy
reset, so the final phone/proxy path must also be checked after deployment.

Deployed at 2026-10-15 15:19 EDT as image
`sha256:9c7e577f151955cb47c705fa7cfa400d3a24173aa84174a637340d83630e20f5`.
Formatter, Clippy and 135 backend tests passed (2 opt-in tests ignored). Readiness
and the LAN HTTP endpoint passed; requested one phone sync to verify the proxy path.

Phone verification succeeded: the user confirmed normal sync at approximately
15:21 EDT. Post-deploy logs showed 19 full 1,000-record batches committed; recent
batches took 822–834 ms with writer-lock times of 146–156 ms. No new Caddy warning
or error appeared after deployment during that check.


### Session renewal deployment (2026-10-15)

Schema 4 adds hashed rotating refresh credentials and a bounded previous-access grace period.
Existing session expiry is preserved. The attempted full pre-migration backup exceeded
available disk space and was removed; no usable snapshot was created. The upgrade itself
runs in one SQLite transaction. Do not downgrade to a schema-3 binary against the upgraded
database; use a forward fix or an independently verified backup. New iOS build 15 renews
at 30-minute intervals while running; inactive sessions expire after 90 days. Authentication
errors and timing are logged without credential values. The direct API endpoint stays
`http://192.168.1.20`; Caddy remains stopped.


### Apple Health 409 on final sample types (2026-10-15)

Observed repeated `health/sync` 409 responses at 23:23–23:24 UTC while connect and
coverage remained 200. The user confirmed `Health record changed without a new
source version`. Added strict support for Apple raw archive representation variants:
only archive data may differ; all other structured fields must match. Both raw
revisions are retained under one logical record, and the batch remains idempotent.
Real structured conflicts still reject with a safe type-only diagnostic. No schema
change or iOS update is needed for this server fix. Image:
`sha256:79d144e6a4b6cd426b1c34f2308ed385597528e3263c43191d08d1c98d0e5c4d`.
Validation: 140 backend tests passed, 2 ignored, formatter and clippy passed.
Device retry is required to confirm whether the blocked batch is an archive-only variant.

Device retry confirmed 2026-10-15 19:39 EDT: the blocked batch contained two
HKCorrelationTypeIdentifierFood archive-only variants, both preserved successfully.
Following requests continued with Active Energy; 313 sync requests returned 200
in the inspected window, no 4xx/5xx. Recent 1,000-record batches took about 0.9s
including request preparation and published zero new raw revisions (existing
records deduplicated). Sync is progressing; completion has not yet been observed.

2026-10-15 metric-history fix: deployed image
`sha256:2071e47d9d693e0afecd89333b829380415431a4eca79fedc530faede3a7a0c1`.
All 17 laboratory catalog metrics have a synthetic plotting regression matrix.
Unassigned clinical records no longer inflate every metric's excluded count;
WBC count-unit aliases are normalized by lab-units-v4. No raw data or schema change.
Full image validation: 141 passed, 2 ignored; formatter and clippy passed.

2026-10-15 dynamic clinical Browse: deployed
`sha256:23e89e4cae8d2edfec32d3490ee3c2a818518b6bc9acb218fa192eed67ed4e75`.
The new authenticated metrics/browse read discovers available clinical lab items;
no database migration or raw-data rewrite. Full image validation passed 142 tests
with 2 ignored, plus formatter/clippy. The read-only live-data diagnostic yielded
76 items / 309 result rows and 25 calcium chart points in an isolated fixture.

## Storage and sync investigation — 2026-10-20

The live `192.168.1.20` database had 11,269,751 4096-byte pages (46.16 GB),
with only 58 free pages. `dbstat` attributed 21.50 GB to `health_records` and
21.50 GB to `health_revisions`; this was live content, not unreclaimed deletes.
There were 4,042,628 logical rows, 223 tombstones, and 4,042,416 original revisions.
No report files were present at the start of this migration. A 500-row sample
contained 1,384,736 JSON bytes, of which 1,046,952 were opaque Base64 archive data.

A 180-row mixed sync batch spent 62 ms staging files but 57,192 ms holding the
SQLite writer. Wildcard deletion queries could only seek by user/platform,
then scan records across sources because `record_id` followed `source_id` in the
existing index. A source-summary read also took 309.64 seconds. Empty sync batches
spent approximately 110 ms repeatedly counting millions of rows under the writer.
The HTTP server is asynchronous, but its Health route also held the global file
mutation mutex during staging. Independently, iOS used the global busy flag for
Health sync and disabled unrelated UI operations.

Schema 5 adds identity indexes for wildcard deletes, a covering source-summary
index, and transactional counters for active device records. Health staging no
longer holds the report/export mutex; SQLite still rechecks owner, version and
tombstones inside each write transaction. Immutable raw paths and temporary-file
leases retain cleanup safety. iOS build 20 gives Health sync its own task state;
report import and browsing continue concurrently. Session-clearing operations
remain gated until Health sync is paused.

Complete original JSON remains under `raw/<user>/apple_health/<revision>.json`.
The query projections replace opaque secure-archive data with `data_sha256`;
structured values, units, source metadata, series and clinical FHIR remain usable.
Distinct original representations remain archived as revisions without increasing
the logical record count. Exact retries remain idempotent. Existing originals/revisions are retained. Report source names select
folders such as `raw/<user>/function_health/<file>.pdf`; images use the same source
folder. Unnamed uploads use `raw/<user>/unassigned/` until their source is saved.

Offline migration (API must be stopped) is:

```bash
python3 src/archive/compact_storage.py --data-dir /absolute/path/backend_api
```

The compactor takes the existing server lock, checkpoints WAL, builds a separate
schema-5 database, checks every original against its indexed projection, checks
row counts/integrity/foreign keys/current-original links, then atomically replaces
the database. It retains the pre-migration database under
`backups/before-storage-v5-<timestamp>/database.sqlite`. Report originals moved to
source folders also get backup hard links. The backup is deliberately excluded
from active database size; it still consumes disk until separately retired.
No account/password/session reset or phone checkpoint reset is part of migration.

Migration completed on 2026-10-20; service was ready again at 11:44 EDT with
image `sha256:cfb342a587d87e3eac942e57afee03d156bdfc794827decb6216796a588700eb`.
The active database decreased from 46,160,900,096 to 23,968,571,392 bytes (48.1%).
All 4,042,628 current rows and 4,042,416 original revisions were retained; the
derived active count is 4,042,405. The old database remains in
`backups/before-storage-v5-1792509460020294209/database.sqlite` and still consumes
space. Originals were verified individually before replacement, followed by
integrity, foreign-key, count and current-original association checks.

The first attempt safely stopped because historical ALTER TABLE migrations left
columns in a different order. The compactor now validates named column definitions
before copying and binds by name; a regression fixture covers that legacy order.
The failed attempt never replaced the original database.

Read-only post-migration measurements on the live dataset: source summary 0.502 s
(previously 309.640 s), active counter 0.018 ms, and one wildcard original lookup
0.042 ms. Query plans use the new covering/identity indexes. These are individual
query measurements, not an end-to-end phone synchronization benchmark. HTTP root
returned 200 and the protected Browse route returned 401 without credentials.
Backend image validation passed 147 tests (2 ignored), formatter and clippy.
Migration/export coverage passed 13 tests. iOS build 20 passed portable/native/UI
validation and uploaded successfully; see the TestFlight runbook.


## Current storage design — schema 7

The earlier schema-5 backup was deleted at the user's request on 2026-10-20,
reclaiming approximately 46.16 GB. The following simplification supersedes the
schema-5 retention design above: only current Health query values and current
reviewed observations live in SQLite. Original files remain indexed for export,
retrieval and deletion; the file catalog contains no copy of the payload.
Apple sampling series and device/archive details are read from originals when needed.
The semantic digest covers structured details omitted from query projections.
Duplicate primary-key/identity and scan-count indexes were removed. Sync receipts,
coverage, tombstones, sessions and required processing queues remain functional state.

The production server accepts only a fresh database or schema 7. Originals use
`raw/<user_id>/<source>/<file>`. The API, cleanup, account deletion and export
validators use only this layout. Previous schemas and source-first paths are not
accepted. `src/archive/compact_storage.py` compacts the current schema only;
old-schema conversion code and fixtures have been removed. It preserves every
current table and raw file, validates counts, foreign keys, database integrity
and current-original pointers, and retains a temporary previous database until
verification. It does not rewrite original files.

Historical schema-6 deployment results follow; the schema-7 deployment below
supersedes their directory layout.

Schema-6 deployment completed on 2026-10-20 at 13:23 EDT, image
`sha256:3fe83968bd336a2f75eb3e4fb69073ef3ec6b11f455eef100bbb9ef98538a1e6`.
The database decreased from 24,157,544,448 to 6,172,086,272 bytes (74.45%).
It retained 4,063,711 Health identities (including tombstones), 4,054,601 original
file entries and 4,054,588 active records. Every copied table count was compared
against the stopped source DB before startup; all matched. Database integrity,
foreign keys and current-original associations passed. Six real categories
(including clinical records, ECG, heartbeat series and environmental audio)
matched their original-file projections. The migration's unchanged-raw-file audit
boundary is documented above. The current compactor enforces source row counts internally.

Post-deployment source summary: 0.400 s; active count: 0.014 ms. Main table sizes:
current Health projections 1,651,523,584 bytes; original-file catalog 1,179,287,552
bytes. Remaining space is predominantly indexes. Both obsolete history tables
are absent. The active raw root contains only `apple_health`; legacy empty source
placeholders were removed and startup did not recreate them. HTTP root 200,
unauthenticated Browse 401, correct image running with zero restarts.

The temporary schema-5 rollback database was removed after verification per the
user's backup-cleanup request, and the empty backups directory was removed.
Filesystem free space was then 85 GiB. Source files and current DB were retained.
Validation: 146 backend tests passed (2 ignored), formatter/clippy passed;
16 offline migration/export/restore tests passed. Logs and size measurements are
in `build/backend_api/validation/helpyourself-storage6-*` and
`storage6-archive-tests.log`. Actual end-to-end phone sync speed was not measured.


## User-first raw layout (schema 7)

On 2026-10-20 the Desk data was converted offline to
`raw/<user_id>/<source>/<filename>`. All 4,054,677 original filenames were
matched against the database catalog before directory moves. The migration
retained all table counts, including 4,063,880 Health identities, 34,227 batch
receipts, 428 coverage rows and the existing session. Database integrity and
foreign-key checks passed; SHA-256 checks of 10 original files matched before
and after the move. Directory renames preserve the original bytes.

The new image is
`sha256:78e74184779d0cc14a9c9e7fabcf4c4f9f5afcd39a1ba1fd35aaee1ca20d1bbe`.
Validation: 146 backend tests passed (2 ignored), Rust formatting and Clippy
passed, 13 archive/restore/compaction tests passed. A synthetic migration with
two users and two sources verified both Health JSON and report-file paths.
No iOS release is needed for this server-side storage change.

The API was ready at 14:47 EDT. Root HTTP 200 and protected raw-list HTTP 401
were verified. All 4,054,677 originals remained after startup cleanup. Old source
directories and the temporary migration database were removed after verification.
The one-use converter was removed; no legacy-layout branch remains in application
or archive code.


## Source display aliases

The source-alias image deployed on 2026-10-20 at 15:15 EDT is
`sha256:ad6969e6dccbb497305fb7c3ed1e59e377f78da363a0b1a6a47101ba8bef4a4a`.
It keeps schema 7 and the user-first raw layout. Aliases use the existing versioned
user preferences; no new table, schema migration or raw-file rewrite is required.
Source catalog names are read from one indexed Health record per distinct source,
plus report contexts, rather than decoding all Health payloads. Preferences remain
exportable and account-scoped. Server validation: 147 tests passed (2 ignored),
formatting, Clippy and Release image build passed; the API returned HTTP 200 after
restart. The browser Data page also provides an alias editor.

## Long Live Health deployment naming (2026-10-20)

Use images `longlivehealth-backend-api:local` and `longlivehealth-backend-ocr:local`,
Compose project `longlivehealth`, executable `/app/longlivehealth`, and the
`LONGLIVEHEALTH_` deployment variable prefix. Desk remains in
`/home/zyin/dev/helpyourself`; its persistent service roots are unchanged. LAN testing
uses direct API HTTP at `http://192.168.1.20`, with no Caddy in the running stack.
Persisted source IDs, raw paths and archive markers remain stable across this product
rename. Existing clients keep using the same server URL and account sessions.

Final-name API and OCR images passed their complete build checks and are deployed.
API tests: 147 passed, 2 ignored; OCR tests: 10 passed. API HTML and OCR health return
HTTP 200, and the web page title is Long Live Health. The API executable is
`/app/longlivehealth`; both containers have the `longlivehealth` project prefix.
API image: `sha256:c6c3ddc2424e101ebab6a32e1a5f0decf011fd3599af511a034993203d788c62`.
OCR image: `sha256:8334e015e63141c53897cf4bccea38961b2d99934f1298a32bfd28e49105263b`.
Provisional-name containers, networks and image tags were removed after verification;
persistent health data and account records were retained.
