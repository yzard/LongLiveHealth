# 完整构建与共享模型清理验收

实际执行仓库根入口 `./build_docker.sh`，退出码为 **0**，用时 **2025.86 秒（约 34 分钟）**。构建期间未修改冻结的 8 个构建输入文件。此次完整运行包含 Android 工具链、ARM64 APK、API 镜像及 OCR 镜像，没有用分步命令代替入口成功。

## 本次补齐

当前 main 中原先没有共享模型/清理辅助实现，入口仍以 `exec docker build` 结束，Android 默认复用长期保留的工作区与 Gradle 卷。本次补上共享层校验和成功退出清理，并将编译放入独立工作区，避免改动正在运行的模拟器所挂载的旧 APK。SDK 容器退出前把本次生成文件归属交回宿主机用户，确保清理可以实际执行。

API 的 Clippy、测试和程序统一使用 release，dev profile 关闭调试符号与增量缓存。Android 保留原有 debug ARM64 APK 交付方式和签名卷；没有改变产品代码或部署现有服务。

## 真实验证结果

- **模型复用**：启动日志明确命中本机共享层；OCR 使用 `model=oci-layout://…` 构建 context。完整日志没有 Hugging Face 模型下载步骤，现有共享层全部文件的大小与 mtime 在运行前后完全一致。
- **模型完整性**：最终 OCR 镜像中的模型 SHA-256 为 `74d2c57145e6ff11d1d2faa79594477f9bc903a611af1fb20218189fbbb77d82`，与固定模型一致。
- **Android**：Gradle unit test、lint、assemble 成功；ARM64 包校验确认 919 个 app-native 方法均已实现。APK 为 151,968,299 字节，SHA-256 为 `a66633c7fad0a9a209ba13a029d4813ffc2b1e57425c8584528a92df699d8b7a`。
- **API**：严格 Clippy、release 构建与 231 项测试通过，2 项既有测试 ignored；镜像内 API CLI 可以执行。镜像中 APK 的 SHA-256 与 dist 导出文件一致。
- **OCR**：isort、Black、10 项测试和实际镜像构建通过。本次没有额外运行 GPU 推理或真实健康报告识别。
- **自动清理**：脚本自行删除本次 builder、其容器与状态卷、本次 Android/Swift/Gradle 工作区、临时缓存和已识别的 Rust target。事后逐项检查均不存在，没有执行手工清理补救或全局 prune。
- **保留范围**：共享 Qwen OCI 层、最终 API/OCR 镜像、版本化 Android 工具链镜像、dist APK、签名卷和验证报告保留。原有服务继续运行，模拟器挂载的旧 APK 校验值不变。旧模拟器/研究工作区、备份及其他项目缓存不属于本次构建创建的资源，未删除。

共享模型位置为 `${XDG_CACHE_HOME:-$HOME/.cache}/shared-model-layers/qwen3.8-27b-nvfp4-ninfer-f0b43ad4/model/`。当前布局仅含固定权重和许可证层；模型元数据不完整时明确失败，不静默重新下载。初始化使用跨项目模型锁，成功构建只清理调用独立 builder 和自己的工作区。编译与清理失败保留诊断并返回非零状态。

## 可复核记录

以下文件保存在忽略的本地输出目录 `build/validation/2026-10-31-full-build/`：

- `build.log`：完整入口日志，包括本机 OCI 模型来源、测试、镜像导入及成功清理。
- `before.json`：运行前构建文件哈希、共享层文件元数据及已有资源。
- `result.json`：退出码、耗时、冻结输入/模型不变校验、资源清理与最终产物检查。
- `observed-build-resources.json`：运行中记录的真实 builder、容器、状态卷和工作区名称。
- `live-artifact-before.json`：现有模拟器 APK 的校验记录。

Android lint 报告另存于 `build/validation/work.loYAhp/android/reports/`。日志含既有 Swift/Skip/Gradle 警告，均未被描述为零警告构建。macOS/Xcode、真机 UI 和生产部署不在这次构建验收范围内。

后续已补齐旧 Android client/research 工作区的迁移与清理，目录从约 14.3 GiB 降至 28 KiB；现有模拟器仅保留产物路径链接。见 [Android 清理记录](2026-10-31-android-cleanup.md)。
