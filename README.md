# gact

gact 是一款专注于 GitHub 动态的 Android 客户端，帮助你跟进关心的活动。

> gact 尚处于早期开发阶段，并非 GitHub 官方应用。

## 功能

- 查看 GitHub 通知，区分未读项目。
- 浏览有权访问的个人、组织及协作仓库，分别查看未关闭的 issue 和 PR 数量。
- 隐藏没有未关闭 issue 或 PR 的仓库，并进入仓库浏览各项内容。
- 按仓库名称或通知标题搜索。
- 阅读 issue、pull request、release 和 check run 的详情。
- 从通知打开对应的 GitHub 页面。
- 在多次启动之间安全保留登录会话。
- 适配全屏、屏幕挖孔、横屏和分屏布局。

## 下载与安装

gact 尚未发布。提供构建版本后，将发布在 [Releases](../../releases) 页面。

- **Release** 构建用于日常使用。
- **Alpha** 构建包含最新变更，稳定性可能较低，应用图标带有 `ALPHA` 标记。

gact 支持 Android 8.0（API 26）及以上版本。安装 APK 前，Android 可能要求允许浏览器或文件管理器安装未知来源应用。

Android 包名为 `com.storytellerf.gact`。

## 登录 GitHub

1. 打开 gact，点击 **Sign in with GitHub**。
2. gact 会在浏览器中打开 GitHub 设备授权页面。
3. 在 gact 中点击 **Copy code**，将代码粘贴到 GitHub 设备授权页面并批准访问。
4. 返回 gact，应用会完成登录并加载通知。

gact 不会要求你在应用中输入 GitHub 密码。会话凭据保存在 Android 安全存储中，退出登录时删除。为显示私有仓库的通知详情，gact 请求 GitHub 的 `repo` OAuth 权限。GitHub 目前没有针对私有仓库内容的只读 OAuth 权限。

## 使用应用

- 在收件箱点击 **Repositories**，查看有权访问的仓库中未关闭的 issue 和 PR 数量。
- 点击仓库，在 **Issues** 和 **PRs** 之间切换；点击条目即可在 GitHub 打开。
- 点击 **Refresh**，加载最新通知或仓库数据。
- 点击通知卡片，查看详情。
- 通知描述支持渲染 Markdown 标题、列表、强调、代码和链接。
- 使用 Android 返回操作逐级退出详情和仓库页面；在根页面执行返回会将应用移至后台。
- 详情加载后，点击 **Open on GitHub** 打开对应网页。
- 请求失败时，确认设备可以访问 GitHub，并检查系统代理设置。

## 常见问题

### 登录后为什么没有通知？

先确认该 GitHub 账户有通知，再点击 **Refresh** 重试。

### 为什么 Open on GitHub 暂时不可用？

某些通知类型需要额外请求才能确定准确的 GitHub URL。请求加载期间或失败时，该按钮保持不可用，避免打开错误页面。

## 隐私与权限

gact 使用网络访问 GitHub API，并通过系统浏览器完成 GitHub 授权。当前版本没有独立账户服务器，也不收集 GitHub 密码。

你可以随时在应用内退出登录，或从 Android 设置清除应用数据。

## 问题反馈

如果通知无法加载、页面显示异常或链接打开了错误页面，请提交 [issue](../../issues)，并提供：

- Android 版本和设备型号。
- gact 构建类型（如果知道）。
- 可复现问题的步骤。
- 可安全分享的截图或错误消息。

不要包含访问令牌、授权码或其他敏感数据。

## 开发

构建说明和检查方法见 [DEVELOPMENT.md](DEVELOPMENT.md)。

项目正在积极开发中，欢迎提交 issue 和 pull request。
