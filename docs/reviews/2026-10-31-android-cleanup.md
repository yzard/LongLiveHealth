# Android 旧构建目录清理

`build/android/` 的占用已从 **15,321,034,752 字节（约 14.3 GiB）降至 28,672 字节（28 KiB）**。旧的 client 和 research 工作区包含重复的 SwiftPM/原生库、生成的 Kotlin/Java、Gradle 与多架构编译产物。

此前完整构建已清理其新建的独立工作区；这两个旧目录未纳入。现在成功退出钩子同时清理已识别的旧目录，单独 Android 构建和组合 Docker 构建都使用此钩子。只有已导出有效 APK 后才允许清理。

现有 `longlivehealth-android-test` 容器只读挂载旧目录中的 APK 子目录。清理将该子目录按原 inode 移到 `dist/android/emulator/ff8f4033ae04/`，包含旧 app-debug.apk 和输出元数据；旧挂载路径仅保留相对符号链接，供现有容器重启时继续解析。其余编译内容已删除，research 目录也已删除。约 157 MB 的旧模拟器产物保留在 dist，估算实际回收磁盘空间约 15.2 GB。

最新构建的唯一交付 APK 仍是 `dist/android/longlivehealth.apk`，没有重新编译、替换或安装 APK。模拟器在清理期间未被停止，其挂载 APK 的 SHA-256 未变化；最新 APK 和共享模型索引也未变化。

## 约束与验证

- 只处理 `build/android/client` 和 `research` 的已识别生成目录。未知工作区、正在被容器挂载的编译目录、路径链接越界、目标冲突或不同文件系统均拒绝删除。
- 与构建入口共用锁；挂载 APK 先迁移，创建旧路径链接失败时恢复原位置，最后清理缓存。签名卷、源码、备份、健康数据和 Qwen 共享层不在清理范围。
- 20 项 development 测试通过，覆盖清理、幂等、目标冲突、作用域、回滚、锁、最终 APK 保留及现有模型复用。
- 用合成 APK 和真实临时 Docker 容器验证了目录迁移后运行中的 bind mount 仍可读，停止并启动同一测试容器后旧源路径仍可读。
- 实际清理成功；之后又执行了真实成功退出钩子，返回 0，重复清理保持幂等。现有模拟器未重启，编译临时目录和测试 builder 均已删除。

运行记录保存在忽略目录 `build/validation/2026-10-31-android-cleanup/`：`before.json`、`result.json`、`cleanup.log`、`bind-proof.json`。本次不需要重新进行 APK 编译或 UI 验收，因为变更只涉及缓存清理与现有产物路径。
