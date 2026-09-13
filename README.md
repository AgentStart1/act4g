# act4g

一款简洁的 Android GitHub 通知客户端，让你可以随时查看 GitHub 上与自己相关的动态。

> act4g 目前处于早期开发阶段，并非 GitHub 官方应用。

## 可以做什么

- 查看 GitHub 通知，并区分未读和已读项目
- 搜索仓库名称或通知标题
- 查看 Issue、Pull Request、Release 和检查任务的详细信息
- 从通知详情直接打开对应的 GitHub 页面
- 安全保存登录状态，下次打开应用时无需重复登录
- 适配全面屏、刘海屏、横屏和较小的分屏窗口

## 下载与安装

请前往 [Releases](https://github.com/storytellerF/act4g/releases) 下载最新 APK。

- **Release**：面向日常使用的正式版本
- **Alpha**：包含最新功能的测试版本，图标带有 `ALPHA` 标记，可能不够稳定

act4g 支持 Android 8.0（API 26）及更高版本。安装 APK 时，Android 可能会要求你允许浏览器或文件管理器“安装未知应用”。

应用包名为 `com.storytellerf.act4g`。旧包名版本会被 Android 识别为另一个应用，登录信息不会自动迁移。

## 登录 GitHub

1. 打开 act4g，点击 **Sign in with GitHub**。
2. 应用会打开 GitHub 的设备授权页面。
3. 确认页面中的授权码并完成授权。
4. 返回 act4g，应用会自动完成登录并加载通知。

act4g 不会要求你在应用内输入 GitHub 密码。登录凭据保存在 Android 的安全存储中；退出登录后，保存的凭据会从设备中移除。

## 使用提示

- 点击 **Refresh** 获取最新通知。
- 点击通知卡片查看详细内容。
- 详情加载完成后，点击 **Open on GitHub** 打开对应网页。
- 如果网络请求失败，请确认设备能够访问 GitHub，并检查系统代理设置。

## 常见问题

### 为什么登录后没有通知？

请确认该 GitHub 账号本身存在通知。也可以点击 **Refresh** 重新加载。

### 为什么 Open on GitHub 暂时不可用？

部分通知需要先向 GitHub 查询准确链接。详情仍在加载或查询失败时，按钮会保持不可用，以免打开错误页面。

### 为什么更新后需要重新登录？

如果从旧包名版本迁移到 `com.storytellerf.act4g`，Android 会将其视为新应用，因此无法读取旧应用保存的登录信息。相同包名和签名的正常升级不会清除登录状态。

## 隐私与权限

act4g 使用网络权限连接 GitHub API，并使用系统浏览器完成 GitHub 授权。当前版本不提供独立的账号服务器，也不会收集你的 GitHub 密码。

你可以随时在应用内退出登录，或通过 Android 系统设置清除应用数据。

## 反馈问题

如果遇到通知无法加载、页面显示异常或链接不正确，
请在 [Issues](https://github.com/storytellerF/act4g/issues) 中反馈，并附上：

- Android 版本和设备型号
- act4g 版本（Alpha 或 Release）
- 问题发生时的操作步骤
- 可以公开的截图或错误信息

请勿提交访问令牌、授权码或其他敏感信息。

## 开发者

act4g 使用 Rust、GPUI Mobile 和 Android Gradle 构建。
Pull Request 会执行 Rust 测试、Clippy、Detekt 和 Android Debug 构建检查。

项目仍在持续完善，欢迎提交 Issue 和 Pull Request。
