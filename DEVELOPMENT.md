# 开发

gact 使用 Rust、GPUI Mobile 和 Android Gradle。CI 在 Ubuntu 24.04 上运行，使用 JDK 17、Python 3.11 或更新版本，以及 `aarch64-linux-android` Rust target。仓库中的 Gradle Wrapper 选择 Gradle 9.8.2，无需单独安装 Gradle。CI 使用 `setup-gradle` 实现缓存和 Wrapper 验证。通过 `cargo install cargo-ndk --locked` 安装 `cargo-ndk`。

## Android 工具链

`android/gradle/gradle.properties` 中的 `gact.ndkVersion` 是 NDK 版本的权威配置，Gradle 和共享的 `.github/actions/setup-android-ndk` action 都从此处读取。使用 `android sdk install ndk/25.2.9519653` 安装该版本。构建前，将 `ANDROID_NDK_HOME`、`ANDROID_NDK_ROOT` 和 `NDK_HOME` 指向同一个已安装的 NDK 目录，例如 `$ANDROID_SDK_ROOT/ndk/25.2.9519653`。原生构建还需像两个 CI workflow 一样设置 `RUST_FONTCONFIG_DLOPEN=on`。

在仓库根目录运行：

```sh
python3 scripts/android-build-checks.py ndk
./android/gradle/gradlew --project-dir android/gradle verifyAndroidNdk detekt --no-daemon --warning-mode all
cargo ndk -t arm64-v8a -P 26 --manifest-path Cargo.toml \
  -o android/gradle/app/src/main/jniLibs build --locked
./android/gradle/gradlew --project-dir android/gradle assembleDebug --no-daemon --warning-mode all
python3 scripts/android-build-checks.py apk android/gradle/app/build/outputs/apk/debug/app-debug.apk
```

工具链检查会比较已安装版本、cargo-ndk 的 clang 和 sysroot 路径，以及 Gradle 解析出的 NDK 目录。APK 检查要求包含 `arm64-v8a` 原生库，检查每个打包原生库的 ELF 架构，并比较 `libgact.so` 与 Rust 输出的 SHA-256。

先构建 Rust，再组装 APK，确保 Gradle 打包的是新库。Debug 构建不需要 OAuth 配置；构建 Rust 时提供 `CLIENT_ID` 可启用 GitHub 身份验证。

## 签名包

使用 `cargo ndk ... build --locked --release` 构建原生 release 库。在运行 `assembleAlpha` 或 `assembleRelease` 前，设置 `GACT_KEYSTORE_FILE`、`GACT_KEYSTORE_PASSWORD`、`GACT_KEY_ALIAS` 和 `GACT_KEY_PASSWORD`。可选的 `GACT_VERSION_CODE` 和 `GACT_VERSION_NAME` 用于覆盖包版本。对相应输出运行上述 APK 检查，再运行 Android SDK 的 `apksigner verify --verbose --print-certs <apk>` 验证签名。本地验证可以使用一次性签名密钥，但这不能验证正式发布的签名凭据。不得提交签名密钥或密码。

## 诊断

### Android 宿主与生命周期回归

应用使用未修改的上游 `gpui-mobile` 提交 `5fd54f4554263ae06753e90472c7f2e2ff5d7495` 及其匹配的 `gpui-pre` 0.3.6 包，没有内置平台补丁。`GactActivity` 管理 SurfaceView，经 `src/android_host.rs` 将生命周期、触摸、硬件按键和 IME 事件转发到 `android::host`。渲染线程、GPUI Application 和 Router 在整个进程生命周期内存活。重建 Activity 会替换 Java 引用和 surface，同时保留原生 UI 状态。进程重启则创建新运行时，并通过 Android Keystore 恢复凭据。

`GactInputActivity` 将上游 InputConnection 示例适配为普通 Activity。凭据 JNI 调用通过当前 Activity 的类加载器解析应用类。保留未使用的 `android_main` 兼容符号，仅因为上游仍链接 android-activity 胶合代码；它从不构造 GPUI。Android 销毁原生 Surface 之前，先通过 `host::surface_destroyed` 释放它。UI 就绪要求 Surface 有效且原生运行时已启动；渲染线程 ID 可用于生命周期诊断。

宿主使用 `adjustNothing`，按逻辑像素转发 IME 与 SurfaceView 的重叠区域。Router 在内容底部预留该区域，同时保持 GPU surface 不变，避免键盘动画缩放已排队的帧。窗口 inset 相对于视图测量，因此已位于 SurfaceView 之外的系统栏不会被重复计入。

先按上文构建原生库，再构建隔离的测试包（不需要账户或现有应用数据）：

```sh
./android/gradle/gradlew --project-dir android/gradle \
  assembleDebug assembleDebugAndroidTest \
  -Pgact.applicationId=com.storytellerf.gact.lifecycle
adb install -r android/gradle/app/build/outputs/apk/debug/app-debug.apk
adb install -r android/gradle/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell am instrument -w \
  com.storytellerf.gact.lifecycle.test/com.storytellerf.gact.ActivityLifecycleTest
adb shell am instrument -w -e scenario keyboard \
  com.storytellerf.gact.lifecycle.test/com.storytellerf.gact.ActivityLifecycleTest
adb shell am instrument -w -e scenario background -e backgroundMs 60000 \
  com.storytellerf.gact.lifecycle.test/com.storytellerf.gact.ActivityLifecycleTest
```

这些测试在已解锁设备上独立运行。默认场景重建 Activity 十次，每次转换后验证渲染线程保持一致且内容已渲染。后台场景从主屏幕返回后，检查 Activity、渲染线程和可见内容保持一致；默认等待 30 秒。成功时打印 `PASS`。原生崩溃会终止 instrumentation，并算作失败。失败时检查 `adb logcat` 和 `dumpsys dropbox --print data_app_native_crash`。测试包与正常的 `com.storytellerf.gact` 应用保持独立。键盘场景验证显示 IME 后视图和缓冲区尺寸不变，隐藏 IME 后恢复已渲染内容。

两个 CI workflow 都使用 `--warning-mode all` 显示各条 Gradle 弃用警告。Java 编译使用 `-Xlint:deprecation`，显示编译器弃用 API 摘要对应的源码位置。这些警告保持可见，包括内置 GPUI Java 辅助代码和第三方构建插件中的警告。

在当前依赖下，Gradle 10 reporting 警告源自 Detekt 1.23.8 的 `DetektPlugin.apply` 调用 `ReportingExtension.file(String)`。Java 警告涉及 `dev/gpui/mobile` 辅助代码中的旧相机、生物识别、位置、媒体会话、麦克风和返回导航 API。更新这些依赖或迁移 API 与选择 NDK 是不同的工作。NDK 安装步骤使用 Android CLI。固定版本的 `android-actions/setup-android` 依赖内部仍调用已弃用的 `sdkmanager` 配置 SDK 并接受许可证；其警告保持可见。

运行 `python3 -m unittest discover -s scripts -p 'test_android_build_checks.py'`，执行包验证器的回归检查。

## Rust 检查

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## 持续集成

Test workflow 在面向 `main` 的 pull request、向 `main` 的 push 和手动触发时运行。Rust 检查与 Android 检查及 debug 构建在不同 runner 上并行执行。两个 job 都必须通过；分支保护应要求 `Rust checks` 和 `Android checks and debug build`，取代原先合并的检查。Android job 先构建 JNI 库，再组装和验证 APK。

Rust 缓存使用不同的 `shared-key`：`host-checks-v1`、`android-debug-v1`，以及用于发布的 `android-release-v1`。Main 上的运行会预热后续 pull request 的验证缓存。Pull request 也会保存供自身后续运行使用的 Rust 缓存。Gradle 保留默认缓存策略：main 写入缓存，pull request 以只读方式恢复。
