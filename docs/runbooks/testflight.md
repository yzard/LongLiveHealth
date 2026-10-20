# TestFlight 内部试机

## 发布准备

- Bundle ID：`dev.helpyourself.ios`；版本与 build number 在 `src/ios/project.yml`。上传已使用过的版本/build 组合前必须递增 build number。
- App 图标来自 `src/ios/Assets.xcassets/AppIcon.appiconset`，1024 × 1024 不透明 PNG。
- `PrivacyInfo.xcprivacy` 声明 app 自身 UserDefaults 与容器内文件时间读取；App Store Connect 的数据收集/隐私问题仍需根据实际部署填写。该文件不表示健康资料不传到服务器。
- app 使用 Apple 系统提供的网络/存储加密，plist 声明 `ITSAppUsesNonExemptEncryption=false`；引入自有加密后须重新评估。
- 保留 HealthKit 与 Clinical Health Records capability；发布 provisioning profile 必须包含相应 entitlement。

## Archive

仅检查 Release 编译与归档（不能上传或安装）：

```bash
./archive_ios.sh --unsigned
```

在 Xcode 已登录且显示付费 Apple Developer Program 团队后，使用该团队的 Team ID（不是 Personal Team）：

```bash
./archive_ios.sh --team YOUR_PAID_TEAM_ID
open build/ios/LongLiveHealth.xcarchive
```

脚本允许 Xcode 自动更新 provisioning。首次签名可能需要账号登录或 Keychain 私钥访问。不要提交证书、私钥、密码、API key 或 provisioning profile。

## 上传与安装

1. 已有 App Store Connect 记录 `Long Live Health`（ID `6822791807`）；Bundle ID `dev.helpyourself.ios`，主语言 English (U.S.)，SKU `helpyourself-ios`。保留同一 App 记录，避免创建新的安装身份。
2. Xcode Organizer 选择签名后的 Archive → Validate / Distribute App → App Store Connect。仅自己试机时可选 TestFlight Internal Only。
3. 等 Apple 处理 build，解决 export compliance/处理错误，再在 TestFlight 创建内部测试组，将自己的 App Store Connect 用户和该 build 加入组。
4. iPhone 安装 Apple TestFlight，接受邀请并安装。无需公开 App Store 上架。外部测试有不同审核流程。
5. 真机填写可从手机访问的实际 Long Live Health HTTP 或 HTTPS 服务器地址。`localhost:18765` 仅是 Mac 模拟器合成测试服务，手机上的 localhost 不指向 Mac，也不提供真实账户验证。

## 无已登记设备时的分发流程

普通自动签名 Archive 需要开发 profile；团队尚未登记设备时会失败。可以使用无签名 Archive，再附上项目实际 entitlement，由 Xcode 云端分发签名：

```bash
./archive_ios.sh --unsigned
./export_testflight.sh VD5S2ZPJ92 --upload
```

省略 `--upload` 只导出 IPA。脚本先保留 HealthKit entitlement，再导出并检查最终 IPA 的签名中包含 HealthKit、health-records、beta-reports-active，且 get-task-allow 为 false，之后才上传。仅检查 embedded.mobileprovision 不够：profile 允许的能力不代表 app 最终签名实际包含它们。

app target 显式设置 `TARGETED_DEVICE_FAMILY=1`，避免 XcodeGen 的 target 默认值覆盖 project 级别的 iPhone 限定，触发 iPad multitasking 方向校验。

## 2026-10-14 发布状态

开发者门户已确认 Individual 会员生效，Team ID `VD5S2ZPJ92`。已创建 App Store Connect app 和 `Personal iPhone Testing` 内部组，添加账号持有人并开启自动分发。build 1 被 Apple 上传校验以 iPad 方向配置拒绝；已修正为仅 iPhone，0.1.0 (2) 于 2026-10-14 上传成功并完成 Apple 处理。App Store Connect 已显示 **Testing**，有效期 90 天，已关联 `Personal iPhone Testing`（1 位测试者）。可以通过本人 TestFlight 邀请安装。

记录：https://appstoreconnect.apple.com/apps/6822791807/testflight 。分发 IPA 在 `dist/ios/TestFlight/LongLiveHealth.ipa`，上传日志在 `build/ios/validation/testflight-upload-2026-10-14.log`。

上传成功之后包装 shell 曾因执行期间文件被编辑而报告多余命令；Apple 的 xcodebuild 已明确返回 Upload succeeded / EXPORT SUCCEEDED。磁盘上的脚本已完成语法检查，不应因为包装 shell 的尾部错误重复上传同一个 build。

未签名 Release Archive 构建成功，10 项 Core 测试通过；该版本并未重新执行全部界面/HealthKit 回归。

官方参考：[上传 build](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds/)、[TestFlight 概述](https://developer.apple.com/help/app-store-connect/test-a-beta-version/testflight-overview/)。

## 0.1.0 (3)：HTTP 与账户管理更新

2026-10-14 已完成 unsigned Archive，并通过 12 项 Core、8 项 AppModel/HealthKit 和 3 项界面测试。包含 HTTP 服务器地址、默认管理员强制改密码、用户管理及导航状态修复。Desk 已部署对应 API，地址 `http://192.168.1.20`。

首次导出因 Xcode 账号凭据失效而失败；重新登录后，2026-10-14 18:03（America/New_York）重试成功。0.1.0 (3) 已完成分发签名和 HealthKit entitlement 检查，Apple 返回 `Upload succeeded` / `EXPORT SUCCEEDED`。上传完成时状态为 processing，尚未确认 build 3 已进入 Testing；内部测试组已配置自动分发。成功日志：`build/ios/validation/accounts-testflight-retry-2026-10-14.log`；首次失败日志保留在 `build/ios/validation/accounts-testflight-2026-10-14.log`。

## 0.1.0 (4)：可暂停的手动 Health 同步

2026-10-15 11:31（America/New_York）上传成功，Apple 返回 Upload succeeded / EXPORT SUCCEEDED；上传结束时处于 processing，尚未确认 Testing。移除启动及前台自动 Health 同步，显示同步阶段和已确认记录数，提供暂停并保留检查点；取消不显示失败弹窗。12 项 Core、9 项原生 AppModel/HealthKit、3 项界面测试通过，Release/Archive 构建及分发 HealthKit entitlement 检查通过。日志：`build/ios/validation/health-pause-upload.log`。

## 0.1.0 (5)：每批最多 1,000 条 Health 记录

2026-10-15 11:39（America/New_York）上传成功，Apple 返回 Upload succeeded / EXPORT SUCCEEDED，结束时为 processing，尚未确认 Testing。HealthKit 查询与上传上限统一为每批 1,000 条，保留按 40 MiB 提前拆批与断点确认；Desk API 同步升级。13 项 Core、9 项 AppModel/HealthKit、128 项后端测试通过（1 项既有负载测试跳过），Release/Archive 和 HealthKit 分发权限检查通过。日志：`build/ios/validation/batch1000-upload.log`。

## 0.1.0 (6)：启动自动同步与 HealthKit 后台投递

2026-10-15 11:58（America/New_York）上传成功，Apple 返回 Upload succeeded / EXPORT SUCCEEDED，结束时为 processing，尚未确认 Testing。恢复启动/前台自动同步，接入 HealthKit observer 后台投递及有限 UIApplication 执行时间；锁屏/到期保留断点，主动暂停按账号持久化。每批上限仍为 1,000 条。最终 IPA 签名已验证 HealthKit、health-records、background-delivery、beta-reports-active，且无 get-task-allow。13 项 Core、11 项原生和 3 项界面测试通过；真机锁屏与系统后台唤醒仍待用户验收。日志：`build/ios/validation/background-upload.log`。

2026-10-15：0.1.0 (7) 上传成功，Apple 后续处理状态未验证。包含服务器累计去重计数和三行同步状态。服务器已部署 schema v3，备份为 Desk build/backups/database-before-sync-total-v3-20261015.sqlite；130 项服务器测试通过（1 项既有测试忽略），15 项 Core、11 项原生模拟器和 9 项 archive 测试通过。实际 186139 条记录统计耗时约 0.006 秒。

2026-10-15 12:45 EDT：0.1.0 (8) 上传成功（Upload succeeded / EXPORT SUCCEEDED）。Apple 后续处理及 Testing 状态未验证。此版本增加 Data → Import → Apple Health Sync 入口分组与详情页标题，并修复未连接 Health 时空同步覆盖已完成状态的问题。最终验证：15 项 Core 测试、12 项原生模拟器测试及登录/导航/Health 授权交互测试通过；发行包确认版本 8，并验证 HealthKit、health-records、background-delivery 和 TestFlight 签名权限。

2026-10-15 13:02 EDT：0.1.0 (9) 上传成功；Apple 后续 Processing/Testing 状态未验证。入口和页面名改为 Apple Health。移除全局同步浮层；页面内统一 Sync Now，Pause Sync 在其下方，同步状态随后显示。Sync Now 和 Data → Import → Apple Health 条目在同步时显示系统旋转指示器。12 项原生模拟器测试及登录/导航/Health 授权交互测试通过，发行包版本和 HealthKit 后台权限验证通过。

2026-10-15 13:24 EDT：0.1.0 (10) 上传成功（Upload succeeded / EXPORT SUCCEEDED）；Apple 后续 Processing/Testing 状态未验证。Archive 标签迁移到 Data → Import → Health Report，Import 菜单保留 PDF/图片、照片和扫描。新增系统 PDF 文档接收和持久待导入队列，确认账号后进入原上传/OCR流程，重试保持稳定上传 ID。17 项 Core、13 项原生模拟器测试及 4 个 UI 场景经修正系统分享定位后分别通过；Safari 的分享菜单已实际验证 Long Live Health 入口及文件接收。

## Build 14 — background uploads and keep-awake (2026-10-15)

Uploaded 0.1.0 (14) at 15:40 America/New_York; Xcode reported Upload succeeded and
processing. Adds file-backed system background uploads for reports and nonempty
Health batches, durable acknowledgements, retry coalescing and logout cancellation.
Apple Health keeps the screen awake only while the page is active and syncing;
pause, completion and navigation restore sleep. The 1,000-record limit remains.
OCR still runs on the server; locked HealthKit reads still await unlock.
Validation: 19 Core tests, 14 native simulator tests, four UI workflows, Release,
archive and distribution HealthKit entitlement checks. Physical lock/scheduling
behavior needs device verification. Build 13's canceled batch-size experiment
was never uploaded.


## Build 15 — persistent login and biometric unlock

Build 15 adds rotating access/refresh credentials, 30-minute renewal while running, durable
retry after lost refresh responses, and one shared renewal owner for Health, report uploads
and regular API requests. Settings provides Face ID / Touch ID unlock with device-passcode
fallback. Background import does not require an interactive biometric prompt.

A password login is needed once to obtain the new refresh credential; prior builds did not
save one. After that, normal use extends the session; 90 days without renewal, account
revocation, password changes or explicit logout require login again. Face ID hardware and
background timing still require real-device validation.


Build 15 validation: 19 Core tests, 18 native tests, four UI workflows, unsigned device Release,
archive, and distribution entitlement checks passed. Signed IPA: `dist/ios/TestFlight/LongLiveHealth.ipa`.
Upload attempted 2026-10-15 18:49 EDT but Xcode returned `Failed to Use Accounts` /
`missing Xcode-Username`; the Mac was locked when inspecting Xcode. Upload is not confirmed.
Unlock the Mac and retry the existing archive before asking for account reauthentication.

2026-10-15 19:13 EDT retry after unlock also returned `Failed to Use Accounts`. Xcode Apple Accounts sign-in window opened for user authentication; build 15 is still not uploaded.

Build 15 upload succeeded after account reauthentication on 2026-10-15 at 19:16:24 EDT. App Store Connect returned `Uploaded package is processing`, `Upload succeeded`, and `EXPORT SUCCEEDED`. TestFlight availability is pending Apple processing.

Build 16 (0.1.0) uploaded successfully on 2026-10-15 at 20:08 EDT.
Apple reported the uploaded package processing; TestFlight availability is not yet
confirmed. This build adds one-page HealthKit prefetch overlapping the current
batch upload and uses `Syncing <cumulative unique count> - <type>` status text.
See the iOS runbook for pipeline durability and validation details.

Build 17 (0.1.0) uploaded successfully on 2026-10-15 at 23:39 EDT.
Apple reported the package processing; availability in TestFlight is not yet
confirmed. This build adds data-driven Browse → Lab Results and preserves clinical
text/bounded results in item histories. The matching backend is already deployed;
users can update in place without repeating Health import.

Build 18 prepares metric detail ordering, unit normalization and duplicate
historical-row fixes. Server normalization applies to existing archived results;
no Health resync or checkpoint reset is required. Read-only projection audit:
76 clinical items, 309 original rows retained, zero items with multiple chart
units after normalization of the currently stored data.

Build 18 validation passed: 144 backend tests (2 ignored), formatting and
Clippy; 23 portable iOS tests, 18 native tests and 4 UI flows; device Release
build, archive and HealthKit distribution-signing checks. Final backend image
`sha256:6f89da9244667ae57255318755b3ab319753b199f81f704e42dad4e08e7be11e`
is deployed directly at `http://192.168.1.20`; readiness confirmed with HTTP 200
and the protected browse endpoint rejects unauthenticated requests with 401.
The apparent original/normalized duplicate rows came from rendering clinical
history records and clinical chart points in the same list. A real pair with
different specimen identifiers remains separate even at the same timestamp.

Build 18 (0.1.0) uploaded successfully on 2026-10-16 at 00:17 EDT;
App Store Connect reported the package was processing.

Build 19 combines visible in-app PDF importing with automatic Health sync
throttling. Regression coverage includes empty-state file-picker navigation,
relaunch-persistent cooldown, concurrent automatic trigger coalescing and manual
sync bypassing the automatic interval. No backend deployment or Health resync is
required for these client changes.

Build 19 validation: 23 portable tests, 20 native tests (including automatic
sync cooldown and manual/concurrent sync regressions), and all 4 UI flows passed.
Archive and TestFlight/HealthKit distribution entitlement checks passed.
Build 19 device Release compilation also passed. The first upload attempt on
2026-10-16 returned App Store Connect HTTP 500 while downloading app information;
this is separate from successful archive/signing validation. Upload is retried
without changing the build or resetting account credentials.
The build 19 retry ended at 2026-10-16 09:45 EDT with exit 70:
"Account credentials have expired. Open Xcode's Accounts settings, sign in to
your account, and try again." App Store Connect had also returned HTTP 500
while creating an asset-upload record. Build 19 is NOT confirmed uploaded.
The signed IPA and archive are ready; reauthenticate Xcode and retry upload
without rebuilding. Logs: build/ios/validation/upload19-retry.log.

Build 19 (0.1.0) upload succeeded after account reauthentication on
2026-10-16 at 13:46 EDT. This supersedes the earlier upload failures above.
App Store Connect returned `Uploaded package is processing`, `Upload succeeded`,
and `EXPORT SUCCEEDED`; TestFlight availability still depends on Apple processing.
Success log: `build/ios/validation/upload19-success.log`.

Build 20 (0.1.0) uploaded successfully on 2026-10-20 at 10:57 EDT.
Apple reported `Uploaded package is processing`, `Upload succeeded`, and
`EXPORT SUCCEEDED`. TestFlight availability still depends on Apple processing.
This build separates Health sync from the foreground operation gate so report
import and refresh remain usable. Validation: 23 portable tests, 20 native tests,
4 UI flows, device Release build, archive and HealthKit distribution entitlements.
The initial native run exposed an obsolete test expecting sync to set global
busy; its expectation now verifies independent sync plus preserved pause state.
The final native rerun passed. Source/UI code did not change after the UI flows.

Build 21 (0.1.0) validation completed on 2026-10-20: 23 portable tests,
22 native tests, 4 UI workflows, device Release compilation and archive passed.
It fixes modal connectivity errors during automatic foreground recovery and
removes the historical-review UI to match current-only database storage.
Export/upload is currently blocked: Xcode reports `missing Xcode-Username`,
`No Accounts`, and no usable iOS Distribution signing identity for the team.
The archive contains build 21; it has NOT been confirmed uploaded. Restore Xcode
account access and rerun `./export_testflight.sh VD5S2ZPJ92 --upload` without rebuilding.

Build 21 (0.1.0) uploaded successfully on 2026-10-20 at 14:39 EDT after
account access was restored. This supersedes the earlier credentials block.
Distribution signing and HealthKit entitlements passed verification. App Store
Connect reported `Uploaded package is processing`, `Upload succeeded`, and
`EXPORT SUCCEEDED`. TestFlight availability still depends on Apple processing.
Success log: `build/ios/validation/upload21-success.log`.


Build 22 (0.1.0) prepared on 2026-10-20: Data → Source aliases provides
account-owned display names across summaries, charts, histories and source pickers.
Aliases preserve source IDs, chart grouping and deduplication; clearing a name
restores the original label. Saves patch retained views without refetching summaries.
Validation: 25 portable tests, 23 native tests and four UI flows passed, plus device
Release compilation and archive. The first PDF share handoff did not open its sheet;
the targeted rerun passed without product changes. The final targeted run also
verified alias save/discard, unchanged summary-request counts and one save request.
The archive's CFBundleVersion was explicitly verified as 22.
Export at 15:19 EDT failed because Xcode could not read account credentials
(`missing Xcode-Username`, `No Accounts`). The Mac was confirmed locked.
Build 22 was NOT uploaded and is superseded by build 23 below.
Logs: `build/ios/validation/build22-initial-tests.log`, `build22-targeted-tests.log`,
`archive22.log`, and `upload22-blocked.log`.

Build 23 (0.1.0) uses the final product name **Long Live Health**. Apple accepted
and saved this name in App Information on 2026-10-20. The same App record
(6822791807), bundle ID and SKU are retained. The final-name rebuild, validation
and upload are in progress; prior provisional-name build artifacts are superseded.

Final-name validation completed: 26 portable tests, 23 native tests and all four
UI workflows passed. The first final-name UI run encountered a non-hittable password
field and cascading state failures; after restarting the dedicated simulator, the
entire native/UI suite passed without product or test changes. The result bundle is
`build/ios/DerivedData/Logs/Test/Test-LongLiveHealth-2026.10.20_15-49-14--0400.xcresult`.
Release archive and distribution export passed; HealthKit, clinical records,
background delivery and TestFlight entitlements were verified. Build 23 retains
`dev.helpyourself.ios` and uses the exact display name `Long Live Health`.
Validation logs: `build/ios/validation/build23-final-tests.log`, `archive23.log`,
`sign23-success.log`; initial interrupted run: `build23-first-run.log`.

**Build 23 uploaded successfully on 2026-10-20 at 15:59 EDT.** Xcode reported
`Uploaded package is processing`, `Upload succeeded`, and `EXPORT SUCCEEDED`.
This final Long Live Health build supersedes build 22 and all provisional-name
build 23 artifacts. Signed IPA: `dist/ios/TestFlight/LongLiveHealth.ipa`.
Upload evidence: `build/ios/validation/upload23-success.log`.
Apple processing is separate from upload; installation availability is not yet confirmed.
App Store Connect independently confirmed `Long Live Health`, version 0.1.0 build 23,
with status `Processing` after upload.
