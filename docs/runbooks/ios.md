# iPhone 构建与试用

源码在 `src/ios/`，SwiftUI + HealthKit + PDFKit/VisionKit，目标 iOS 17 起，Swift 6 严格并发。Core 的 Swift Package 使用工具链 6.2；完整构建使用带对应 SDK 的 macOS/Xcode（例如 Xcode 26）和 XcodeGen。

2026-10-02 已在 Xcode 27 / iOS 26.4 完成模拟器构建、Core 与 Apple SDK 测试、HealthKit 血糖保存主流程和 unsigned iPhone Release 构建。具体通过范围及仍需验收的内容见 [iOS 模拟器记录](../validation/ios-simulator-2026-10-02.md)。

## 统一构建入口

在 macOS 安装 Xcode 与 XcodeGen，选择完整 Xcode 工具链后，从任意目录执行仓库的 `build_ios.sh`：

```bash
./build_ios.sh
# 可选：指定已安装的模拟器执行 Apple SDK 测试
./build_ios.sh --test-destination 'platform=iOS Simulator,id=YOUR_SIMULATOR_ID'
```

脚本运行 Core 测试，将 XcodeGen 工程生成到 `build/ios/`，编译模拟器测试目标，再构建 unsigned iPhone Release App。最终产物为 `dist/ios/LongLiveHealth.app`，中间文件在 `build/ios/DerivedData/`。这是未签名设备 App，不是可直接安装的 IPA；安装仍须自己的开发团队、Bundle ID 和 provisioning。

Linux 可执行 `./build_ios.sh --core-only`：在 Swift 6.2 容器内做全部 Swift 语法解析和可移植 Core 测试。未传此选项时 Linux 明确拒绝完整构建，不把 Core 检查当成 iOS build。

所有源文件在 `src/ios/`，测试镜像在 `tests/ios/`；没有旧平台树或兼容副本。生成工程后可在 Xcode 打开 `build/ios/LongLiveHealth.xcodeproj` 做签名和真机调试。XcodeGen [配置文档](https://github.com/yonaskolb/XcodeGen/blob/master/Docs/Usage.md)是相关设置的参考。

模拟器测试使用 ad-hoc 签名和自动启动的 loopback 合成 API（Python 3，端口 18765），不连接生产账户。生成的 Info.plist 与 entitlements 位于 build/ios，测试退出会停止合成 API。直接在 Xcode 运行测试时，先启动 `python3 tests/ios/simulator_fixture.py`。真实记录与 Watch 来源尚未计入模拟器通过范围。

## 试用流程

1. 输入 HTTP 或 HTTPS 服务器地址和凭据。首次使用 admin/admin 登录后必须改密码；设置页可改密码，admin 可管理普通账号。HTTP 地址由部署者选择，HTTPS 仍不跳过证书验证，不跟随重定向转发 token。
2. Reports 导入 PDF/照片或扫描纸质报告。PNG/JPEG 保留所取文件原字节；HEIC/HEIF 同时上传原件和 JPEG 识别副本。扫描器返回的每页 PNG 单独上传，当前各页分别是一份报告。上传前持久化全部待上传字节和 UUID；响应丢失重用 UUID。待上传行可左滑丢弃。
3. 进入报告看原件、提取状态、页级文字、完整 OCR/解析回复和候选。点选候选，核对原值、单位、采样日期、参考区间、指标映射与出处，再选 Confirmed。也可人工加行、保留 Pending 或 Rejected，全部由 API 保留历史。
4. Trends 选择指标，可同时比较第二项，各自使用独立纵轴。点击图表附近的点或日期行回到原件页。
5. Health 连接权限并同步。视力处方与药物需要在“Select prescriptions and medications”中逐条选择，普通前台同步不会自动弹出这些选择页面。每个查询页拆分出的全部批次先落盘，全部确认后推进锚点；下次自动或手动同步继续。样本 secure archive、可读序列和快照都归档到 API。类型随 OS/权限动态可用，范围与缺口见支持矩阵；临床记录读取另需 Health Records entitlement 和设备支持。
6. Insights 仅在服务器启用 analysis 后允许创建个人血脂回顾，查看引用、缺失资料、其他解释和就医问题，记录反馈。下拉刷新查看后台任务结果。
7. Settings 创建、刷新、下载并分享完整 ZIP，也可删除导出、报告或账户。

已确认 glucose 项提供显式 Save to Apple Health：仅接受精确数值及 mg/dL 或 mmol/L。用户须核对真实采样日期和时间；先将该时间作为新修订保存 API，再请求 HealthKit 写权限并保存血糖样本。稳定 sync identifier/version 用于重试及修订去重。日期级旧修订仍保留，不默认伪造采血时间。若已启用 Health 同步，保存后尝试回读归档；否则需用户开启同步。其余报告项目保持 API 全量归档，不将 LDL/HbA1c 映射成错误的膳食类型。此写回流程已通过 Apple SDK 和模拟器真实 HealthKit 授权/保存测试；真机验收仍未完成。

服务器报告删除与 Apple Health 样本删除是独立操作；当前删除报告不会替用户删除已经写入 Apple Health 的样本。

## 本机数据

token 存 Keychain，ThisDeviceOnly，不进入普通偏好文件。缓存按服务器地址和 user_id 的摘要分目录，包含最近刷新时间、已打开原件/报告和待上传队列，排除 iCloud 备份并使用 iOS 文件保护。成功刷新列表时清除服务器已删报告的缓存。

退出立即清理本机缓存和 Keychain，并尽力撤销服务器会话；离线撤销失败时旧服务器会话等到期或管理员撤销。本机无云端强制清除能力：离线设备保留的旧缓存需重新连线刷新、退出或删除 App；手动分享出的 ZIP 不会被远程删除。

HealthKit 读取授权无法用授权状态 API 判定；用户拒绝和没有样本可能同样呈现为空。实现显示未知而非声称已授权，依据 [Apple 授权文档](https://developer.apple.com/documentation/healthkit/authorizing-access-to-health-data)。支持 HealthKit observer 后台投递（向系统请求 hourly 频率），启动及前台/解锁机会自动同步；不承诺后台实时或无限运行。锁屏数据保护和系统执行时间限制会暂停同步并保留断点。


## 2026-10-14 skills 对齐审查

适用规则：add-modify-codebase、general-coding、write-swift、project-structure、naming-conventions、restful-api-design、axum-server、sql-coding、docker-build，以及 apple-design 的导航/原生交互原则。Python 夹具和归档工具按 code-formatting 检查。其余技能用于图像、文档、网页托管、表格、插件、宠物、C++、Python 服务、Expo 或动画专项等，本次没有对应实现；不将这些专项规则强加到 Swift 应用。

已修正：HTTP 地址和 ATS 配置一致；默认管理员强制改密码由服务端和客户端双层执行；账户切换重建视图状态；请求完成以 session token 检查所属账号；网络故障才允许报告缓存回退。默认密码会话不发起健康同步或上传。首次加载任务保留查询状态，切回标签或关闭编辑器不重复读；编辑器使用 sheet 并保护未提交输入；手动条目保存/删除按服务端记录局部更新；派生视图仅在真实数据变化时重新计算。

保持：Swift 6 完整并发检查、Approachable Concurrency、UI MainActor 隔离，Core 明确 nonisolated；HealthKit 多回调适配器的 unchecked Sendable 由锁保护缓冲、查询与 continuation，取消和完成只恢复一次。保留 HealthKit 原始样本、分页/批次与 checkpoint，不声称空结果表示已授权。标准 SwiftUI 字体、导航、控件和 sheet 保留平台辅助功能；没有引入新的自定义动画或未经测量的线程池。

验证记录以本轮测试结果为准；模拟器无法代替实际手机传感器、真实临床 Health Records、相机和生产 OCR 准确率验证。

本轮验证完成：12 项 Core 测试、8 项 AppModel/HealthKit 测试、3 项 XCTest 界面流程全部通过。界面回归覆盖默认 admin 强制改密码、登录/标签切换、编辑取消后无多余读取、HealthKit 授权、报告编辑/证据/关联/分析。后端 127 项测试通过、1 项已有负载测试跳过，Clippy 无警告；归档工具 9 项、前端 12 项测试通过。另用独立 Docker 数据目录验证默认账号、普通重启保留密码和 RESET_ADMIN_PASSWORD=true 重置。测试夹具不使用真实健康资料。

日志：`build/ios/validation/accounts-ios-2026-10-14.log`；测试结果：`build/ios/DerivedData/Logs/Test/Test-Helpyourself-2026.10.14_11-52-53--0400.xcresult`。

## 2026-10-15 Health 同步暂停修复（build 4）

此前 App 每次启动/回到前台都会自动同步全部可见 Health 数据，同时占用全局 Working 状态。首次历史导入每页 10 条，长时间没有可见进度，也没有取消入口；重开会再次续传。此版本移除启动/前台自动健康同步，改由 Health 页明确发起。同步浮层显示当前类型和本次已获服务器确认的记录数（重放批次也计入，并非新增唯一记录数），提供 Pause sync。暂停取消拥有的任务，释放忙碌状态，保留已持久化批次和锚点；正常取消不弹出操作失败。HealthKit 查询和快照的错误转换先检查取消，避免把暂停记录成读取失败。

服务器诊断确认已有 29,330 条归档记录，3,582 次成功同步请求；曾发生一次 coverage 响应连接中断。尚未获得手机端网络诊断，因此不把本次界面/生命周期修复宣称为该网络超时的根因修复。

验证：12 项 Core、9 项 AppModel/HealthKit（新增暂停释放忙碌状态及保留检查点）、3 项界面流程通过；iPhone Release 和 build 4 Archive 成功。界面流程覆盖主应用修改；随后对最终取消处理和新增用例重新运行全部 9 项原生测试。日志保存于 `build/ios/validation/health-pause-*.log`。

## 2026-10-15 Health 批量上限（build 5）

HealthKit 每页读取及每个上传批次最多 1,000 条；一次手动同步可以连续处理多个批次。快照也使用相同上传条数上限。请求仍受 40 MiB 限制，单条 payload 上限 32 MiB，超大的批次按字节提前拆分；全部批次确认后才推进锚点。组包只逐条编码计算字节数，避免每加一条就重新编码整页。已有小批次检查点可继续重放。

验证：13 项 Core、9 项原生 AppModel/HealthKit 通过；服务端 128 项测试通过、1 项既有负载测试跳过，fmt/Clippy 通过。新增覆盖 2,001 条拆为 1,000/1,000/1、转义及 Unicode 字节边界、服务端接收 1,000 条与拒绝 1,001 条不写入。Release/Archive 成功；本次没有重复运行未修改的界面流程。Desk 对应 API 已部署并通过状态检查。

## 2026-10-15 后台与启动自动同步（build 6）

按用户最新要求恢复已连接账号的启动/前台自动同步，并在 app delegate 启动时安装 HealthKit observer。增加 `com.apple.developer.healthkit.background-delivery` entitlement，并在最终分发 IPA 检查该签名权限。对支持的非临床、非关联、非单独选择的样本类型请求 hourly 后台通知；临床和逐项选择仍遵守原授权流程，不在后台弹授权。同步获得有限 UIApplication 后台执行时间，结束及到期都释放；到期或保护数据不可读时取消所属任务，保留批次和锚点，自动同步错误显示状态而非普通失败弹窗。

解锁回调和前台回调恢复同步；主动 Pause sync 按账号落盘，直到手动 Sync now/Connect 再启用。每批 1,000 条及 40 MiB 限制不变。后台通知 completion 通过锁保护的一次性容器交接，在本次处理结束后确认；并发通知合并。手机重启、强制退出、系统调度和数据保护仍受 iOS 控制，不保证开机立即运行或锁屏持续读取全部历史。

参考：[Apple 后台 observer](https://developer.apple.com/documentation/healthkit/executing-observer-queries)、[Health 数据隐私与锁屏](https://developer.apple.com/documentation/healthkit/protecting-user-privacy)。真机后台唤醒与锁屏时长必须在手机验收，模拟器只验证逻辑、SDK 构建与正常授权流程。

验证：13 项 Core、11 项 AppModel/HealthKit、3 项界面流程通过，随后对最终任务所有权/取消处理重跑 11 项原生测试通过；最后只细化主动暂停提示文案并重新 Archive。新增用例验证 HealthKit 保护数据错误识别、通知一次性确认、暂停设置跨 AppModel 恢复后禁止自动请求。有限后台执行、真实锁屏读保护和系统唤醒仍需真机验证。

Build 7 同步进度分三行：`Syncing`、`Active Energy · 29,330`、`Pause Sync`。计数取服务器当前有效设备记录的跨同步累计去重总数，不附加 imported / Records Confirmed。无需额外扫描 HealthKit；服务器使用 schema v3 部分索引统计。必须先更新服务器。

Build 8 将 Data 顶部入口分为 Import 与 Export：Import 内使用 Apple Health Sync 名称，详情页同名；沿用 build 7 的三行同步进度和服务器累计去重计数。

Build 8 同时修复未启用 Health 连接时的空同步：保留“Reviewed glucose saved to Apple Health”等已完成状态，不能停留在 Preparing Apple Health sync。原生回归测试覆盖该路径。

Build 9：入口为 Data → Import → Apple Health。同步状态与 Pause Sync 仅在 Apple Health 页面显示，其他页面不显示同步浮层。首个按钮为 Sync Now（沿用 HealthKit 授权后同步路径；已决定的权限无需重复选择），同步期间 Pause Sync 紧接其下，状态随后显示。Sync Now 和 Data 的 Apple Health 条目右侧使用系统旋转进度指示器，并提供 Syncing 无障碍值；同步结束或暂停后隐藏。移除重复操作按钮，并更新恢复提示。

Build 10：删除 Archive 底部标签，报告列表位于 Data → Import → Health Report。Import 菜单保留 PDF/图片、照片与扫描入口；OCR 和审核流程不变。注册 com.adobe.pdf 文档类型（Viewer / Alternate），接收系统分享或 Open In 的本地 PDF 副本；不处理网页链接。收到文件后存入 Application Support/shared-report-inbox，显示文件和目标账号；用户点 Import 才转入该账号的原有上传队列。未登录、关闭确认页或重启均保留未分配文件，可在 Data 的 Import shared PDFs 中继续或丢弃。限制 20 MiB，拒绝非 PDF 内容；上传重试沿用稳定 ID。

Build 10 验证：17 项 Core 测试、13 项原生模拟器测试通过；4 个 UI 场景分别通过（管理员密码、报告/Health 导航、报告审核编辑、Safari PDF 系统分享）。分享菜单位于独立的 com.apple.SharingUIService 系统进程；UI 测试从该进程选择 Long Live Health，确认导入页收到 shared-report.pdf。

Build 11：移除 Apple Health 中 Last 30 days / Metric 面板。Overview 增加 Health metrics，点 LDL 等指标进入统一历史页；Apple Health、Function Health、诊所等按来源颜色及形状绘制同一张图，下面逐条显示日期、值、来源和原始报告入口。日常指标历史也显示全部已有来源。切换标签或关闭报告编辑器不自动重载，用户可下拉或点 Refresh history 更新。

所有新 PDF/照片/扫描导入先填写 Clinic or laboratory，系统分享的每个 PDF 分别填写名称。账号上传队列保存该名称；上传成功但来源保存失败时保留同一个上传 ID 重试。旧报告可在 Collection context 添加名称；未命名时图中暂用文件名。来源通过现有报告 context 保存，原始 PDF、OCR 证据和原始 Health 记录均保留。只有已审核、可识别单位且有采样日期的报告数值进入图表。

先部署支持 metrics/history/get 和 context.source_name 的服务器，再安装 build 11。Apple Health 的临床导入使用闭集 LOINC 映射，不能识别所有临床编码；未识别数据仍保存在归档。服务器的查询上限和排除规则见 docs/api.md。

Build 11 验证：17 项 Core、13 项原生模拟器测试通过；4 个 UI 场景最终分别通过，包括 Overview LDL 三来源图、必填机构名称、Health 授权、报告编辑及系统 PDF 分享。首次全套分享断言因 Safari 自动追加下载文件编号失败，修正文件名匹配后分享流程通过；同时清理测试留下的未分配合成文件，再跑全部 13 项原生测试通过。新增重试回归覆盖文件上传失败、文件已上传但来源保存失败、空机构名称不入队，以及稳定上传 ID。最终图表日期使用 UTC，避免日期精度的检验结果显示在前一日；Release Archive 编译通过。

服务器最终 fmt/Clippy 和完整测试通过：133 passed、1 个原有负载测试 ignored。新增统一历史回归涵盖三来源 LDL、用户隔离、同步重放/删除、单位换算、非最终/有界/缺日期排除、血糖数量记录及 FHIR 分项、机构重命名与非法名称拒绝。大量临床 JSON 的解码放在服务端限流 CPU 执行器中。

0.1.0 (11) 于 2026-10-15 13:51 EDT 上传成功，Xcode 返回 Uploaded package is processing / Upload succeeded；分发签名已验证 HealthKit、health-records、background-delivery 和 TestFlight 权限。此记录仅确认上传及 Apple 开始处理，不代表已核实 TestFlight Testing 状态。Desk 最终镜像 e66c48b12534 已运行，新接口未登录探测返回 401。

Build 12：Overview 改为 Yesterday 与 Last 7 days 两段摘要，后者为截至昨天的七个完整本地日。按来源分开显示日均值、实际有记录天数和用户睡眠目标达成天数；缺失记录不计为零，也不把不同来源平均在一起。没有配置的参考范围明确显示未设置，不推断临床正常。Customize 保留重要指标选择和个人睡眠目标；移除 Overview 日期选择、指标目录以及 Device trends/Timeline/Breathing practice 等入口。跨日回到前台改变查询日期，同日普通标签切换不重载。

Overview 旁新增 Browse 标签，包含日常指标、全部检验指标和逐项详情入口，保留统一多来源图、报告原件及原有功能。日常详情按来源显示所有值，无选定来源时不再在日期标题错误显示 Not available。睡眠摘要、日常记录和睡眠 session 统一显示 hr/min；睡眠图轴用小时。原始存储单位仍为秒。

Build 12 验证：19 项 Core、13 项原生模拟器测试及导航/Health UI 回归通过；新增覆盖小时分钟进位、缺失/跨窗口日期、来源分离、目标达成数和闰日边界。UI 检查 Overview 移除旧入口/日期选择、睡眠格式、Browse → LDL 多来源详情、五个标签导航和原有报告编辑状态。Release Archive 和最终分发签名检查通过。此次无服务端变更。

0.1.0 (12) 于 2026-10-15 14:07 EDT 上传成功，Apple 已开始处理；尚未核实 TestFlight Testing 状态。范围采用个人目标，不给未配置指标推断正常范围。

### Background transfers and the Apple Health screen (build 14, 2026-10-15)

Report uploads and nonempty Health sync batches use a shared background URLSession
with file-backed request bodies. The system can continue transfers while the app is
suspended; results are persisted before background-event completion is acknowledged.
Business drafts and Health checkpoints still advance only after server acknowledgement.
Request identity includes the origin, session identity and stable operation ID, and
concurrent retries join an existing system task. A 120-second UI wait can expire
without canceling the system upload or deleting the draft. Signing out cancels
transfers and clears their local files. No OCR result is automatically written to Health.

OCR remains a durable server job once the file upload succeeds. Clinic/laboratory
context acknowledgement remains part of the draft workflow; interrupted follow-up
requests resume with the same upload ID. Reopening the app retries drafts, refreshes
server job status and resumes enabled Health sync.

The Apple Health page disables the idle timer only while visible, active and syncing.
Pause, protected-data suspension, completion, tab/page departure and backgrounding
restore automatic screen sleep. The side button still locks the device. HealthKit
reads can become inaccessible while locked; prepared uploads may continue, but new
Health queries wait for unlock or another system-granted execution opportunity.
Background delivery is scheduled by iOS, not an always-running service. Force-quitting
an app can prevent automatic background transfers until the user opens it again.

Platform references: [HealthKit privacy and locked data](https://developer.apple.com/documentation/healthkit/protecting-user-privacy)
and [system background URLSession transfers](https://developer.apple.com/documentation/foundation/downloading-files-in-the-background).
The simulator validates request lifecycle and app navigation, but cannot establish
real-device HealthKit protected-data access or iOS background scheduling guarantees.

Validation for this update: 19 portable Core tests, 14 native simulator tests and
four UI workflows passed; Release and archive builds succeeded. Native coverage
includes a delayed system file upload that survives caller cancellation, concurrent
retry coalescing, report retry identity/source acknowledgement, saved Health batches,
and immediate removal of the screen-awake intent on Pause. Device lock scheduling
still requires iPhone verification because the simulator cannot reproduce HealthKit
protected-data behavior. Distribution entitlements are checked by the export script.

Build 0.1.0 (14) uploaded successfully at 2026-10-15 15:40 EDT. App Store Connect
reported the package processing; Testing availability was not yet confirmed.
Logs are in `build/ios/validation/helpyourself-build14-*.log`.


## Persistent login and device unlock (build 15)

The app renews access and refresh credentials every 30 minutes while running and before API
work when renewal is due. iOS suspension does not guarantee a timer; foreground launch and
background work opportunities perform overdue renewal. Credentials are stored with
AfterFirstUnlockThisDeviceOnly Keychain accessibility; account passwords are not saved.
A refresh owner coalesces requests, persists the successor before sending, retries the same
pair after interruption, and cannot save credentials after the account has signed out.
Network/5xx failures retain login. Definitive 401 preserves account-scoped pending imports
and checkpoints for reauthentication; explicit sign-out still clears this device.
Previously issued sessions have no refresh credential and require one password login.

Settings → Face ID / Touch ID unlock is opt-in. Enabling or disabling requires local device
authentication; enabling requires enrolled biometrics. A restored account and each background
transition lock the UI. Biometrics or device passcode unlock it. An opaque window above sheets
also covers app-switcher snapshots. Cancelled authentication leaves the app locked. This is
local app access protection, not server password authentication: revocation still requires
account login, and background sync retains noninteractive access to its Keychain credential.
Real Face ID hardware behavior must also be checked on iPhone; simulator tests cannot certify it.

Pending next iOS release: the Apple Health status uses `Syncing 29,330 - Active Energy`.
The number remains the server's cumulative unique record count, including earlier syncs;
it is not a count of records checked during the current run. Unknown totals are omitted.

Build 16 changes anchored Apple Health imports to a bounded pipeline: while one page
(up to 1,000 records) uploads, one next page is queried and encoded. The next query
uses the current page's candidate anchor without committing it. Only successful
uploads of every split request advance the durable anchor. The current page remains
in the existing pending checkpoint until completion; prefetched data stays in memory
and is queried again after interruption. Upload failure/cancellation cancels and joins
the prefetch; a prefetch failure cannot discard an already successful upload. Pending
checkpoints from older builds replay before pipelining begins. Snapshot imports are
unchanged. No extra counting scan or server API change is required.
Build 16 validation: 23 portable tests passed (including overlap, upload-failure
cancellation, prefetch-error ordering and already-cancelled tasks); simulator build,
unsigned device Release build, archive and distribution entitlement checks passed.
The first native run failed two existing PDF review-retry assertions; an unchanged
rerun passed all 18 native tests. Both logs are retained in build/ios/validation.
Real-device throughput still needs measurement; no fixed speedup is claimed.

Build 17: Browse → Lab Results loads `metrics/browse` rather than the fixed review
catalog. History charts separate units and list clinical text, bounded and other
non-plotted results with dates, hospital names and Apple Health provenance. Known
laboratory metrics continue to combine with reviewed PDF results. Local cache
supports the new read endpoint; refresh updates the available items after import.
A read-only production-data diagnostic used an isolated temporary database and
verified 309 imported lab results produce 76 items, including 25 plotted calcium
records. Production records were not modified by the diagnostic.
Build 17 validation passed: 23 portable tests, 18 native tests and all 4 UI flows
(including dynamic Calcium discovery and NYP text-result detail). Backend image:
142 tests passed / 2 ignored. Device archive and TestFlight entitlements verified.

Build 18: metric details show charts, metric information/calculation/display
units, then historical values. Clinical history displays normalized units and
expandable original results. Clinical chart points are not repeated as extra
historical rows. Daily metric descriptions document the actual aggregation
(interval union, prorated step sums or arithmetic sample means).

Build 19 adds a prominent Import PDF action at the top of Health Report,
including the empty state. Enter a clinic/laboratory name, then choose PDF from
Files (On My iPhone, iCloud Drive or another Files provider). When Health sync
occupies the operation slot, the screen explains why import is unavailable and
provides Pause Sync. The existing image, photo and scan actions remain available.

Automatic Health sync coalesces triggers during an active run and persists a
per-account/server next-automatic-check timestamp. Successful syncs defer the
next automatic pass by 15 minutes; failures use a one-minute retry interval.
Repeated notifications share one deferred task. Manual Sync Now bypasses this
interval, cancels the deferred task and resumes user-paused automatic sync.
Lock/background interruption releases the interval so unlock can resume from
saved checkpoints. Manual pause and session clearing cancel deferred work.
A deferred task depends on iOS execution opportunities; this is a minimum
interval, not a guaranteed background timer.

Build 20 separates `healthSyncTask` from the foreground `isBusy` gate. Health sync
and report upload/refresh can run concurrently; repeat Sync taps remain blocked.
The native regression performs refresh and report upload while a manual Health
sync is in progress. Pausing still retains checkpoints and clears keep-awake state.
Signing out, deleting the account and changing its password require pausing sync
first so local files and session ownership cannot change beneath the pipeline.

Build 21 separates automatic foreground recovery from explicit user actions.
Initial activation and subsequent unlocks share one recovery task; backgrounding
cancels that foreground task. Automatic replayable operations retry transient
connectivity failures twice (1 s, then 3 s). Exhaustion shows the offline status
without a modal alert, retaining credentials and pending report bytes. Manual
failures and non-connectivity errors remain visible. Successful refresh clears
the status. The duplicate refresh after pending-upload recovery is removed.
The report review no longer offers historical edits: schema 6 retains current
review values and revision counters for concurrent edits, with original evidence.


Source aliases (build 22): Data → Sources → Source aliases lists device and report
source identities. Editing opens a sheet and saves explicitly using the current
preference version. Cancel/discard does not write or refresh the underlying view.
Names are resolved through the account-scoped AppModel cache across summaries,
charts, historical values and source pickers. Chart domains and API selections
retain source keys, even when two aliases are identical. Clearing the field removes
the alias. The cache is persisted under the server/user archive and cleared on logout.

## Long Live Health naming

The product, home-screen name and PDF share destination use `Long Live Health`.
The Xcode project/scheme and Swift module use `LongLiveHealth`; the portable module
is `LongLiveHealthCore`. Build 23 applies this name.
The repository and its working directory remain `helpyourself` by user request.
The shipping bundle ID `dev.helpyourself.ios`, Keychain service, background-transfer
identifier, reminder identifiers and HealthKit sync metadata remain unchanged so an
upgrade retains authentication, queued uploads and deduplication identity. These are
persisted identifiers, not product display names. Manual source IDs and export format
markers also remain stable; manual-source display labels resolve to Long Live Health unless
the user has assigned an alias. No data rewrite or health-history resync is needed.
