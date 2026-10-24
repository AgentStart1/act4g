# Development

gact uses Rust, GPUI Mobile, and Android Gradle. CI runs on Ubuntu 24.04 with
JDK 17, Python 3.11 or newer, and the `aarch64-linux-android` Rust target.
The checked-in Gradle Wrapper selects Gradle 9.4.1; a separate Gradle installation
is not required. CI uses `setup-gradle` for caching and wrapper validation.
Install `cargo-ndk` with `cargo install cargo-ndk --locked`.

## Android toolchain

`android/gradle/gradle.properties` owns the NDK version (`gact.ndkVersion`).
Gradle and the shared `.github/actions/setup-android-ndk` action both read it.
Install that version with `android sdk install ndk/25.2.9519653`. Before building, set
`ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, and `NDK_HOME` to the same installed NDK
directory, for example `$ANDROID_SDK_ROOT/ndk/25.2.9519653`.
Set `RUST_FONTCONFIG_DLOPEN=on`, as in both CI workflows, for the native build.

From the repository root:

```sh
python3 scripts/android-build-checks.py ndk
./android/gradle/gradlew --project-dir android/gradle verifyAndroidNdk detekt --no-daemon --warning-mode all
cargo ndk -t arm64-v8a -P 26 --manifest-path Cargo.toml \
  -o android/gradle/app/src/main/jniLibs build --locked
./android/gradle/gradlew --project-dir android/gradle assembleDebug --no-daemon --warning-mode all
python3 scripts/android-build-checks.py apk android/gradle/app/build/outputs/apk/debug/app-debug.apk
```

The toolchain checks compare the installed revision, cargo-ndk's clang and
sysroot paths, and Gradle's resolved NDK directory. The APK check requires the
`arm64-v8a` native library, checks every packaged native library's ELF architecture,
and compares `libgact.so`'s SHA-256 with the Rust output.

Build Rust before assembling the APK so Gradle packages the new library.
Debug builds do not require OAuth configuration. Supply `CLIENT_ID` while building
Rust to enable GitHub authentication.

## Signed packages

Use `cargo ndk ... build --locked --release` for the native release library.
Set `GACT_KEYSTORE_FILE`, `GACT_KEYSTORE_PASSWORD`, `GACT_KEY_ALIAS`, and
`GACT_KEY_PASSWORD` before running `assembleAlpha` or `assembleRelease`.
`GACT_VERSION_CODE` and `GACT_VERSION_NAME` optionally override package versions.
Run the APK check above on the corresponding output, and run Android SDK
`apksigner verify --verbose --print-certs <apk>` to verify its signature.
Local verification can use a disposable signing key; that does not validate the
production signing credentials. Never commit signing keys or passwords.

## Diagnostics

Both CI workflows use `--warning-mode all` to show individual Gradle deprecations.
Java compilation uses `-Xlint:deprecation` to show the source locations behind
the compiler's deprecated-API summary. These warnings remain visible, including
warnings in bundled GPUI Java helpers and third-party build plugins.
With the current dependencies, the Gradle 10 reporting warning originates in
Detekt 1.23.8's `DetektPlugin.apply` calling `ReportingExtension.file(String)`.
Java warnings include legacy camera, biometric, location, media-session,
microphone, and back-navigation APIs in `dev/gpui/mobile` helpers. Updating these
dependencies or migrating those APIs is separate from selecting the NDK.
The NDK installation step uses Android CLI. The pinned `android-actions/setup-android`
dependency still invokes deprecated `sdkmanager` internally for SDK setup and
license acceptance; its warnings remain visible.

Run the package verifier's regression checks with
`python3 -m unittest discover -s scripts -p 'test_android_build_checks.py'`.

## Rust checks

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Continuous integration

The Test workflow runs on pull requests to `main`, pushes to `main`, and manual
dispatch. Rust checks and Android checks/debug builds run in parallel on separate
runners. Both jobs must pass; branch protection should require `Rust checks` and
`Android checks and debug build` in place of the former combined check.
The Android job builds JNI libraries before assembling and verifying the APK.

Rust caches use separate `shared-key` values: `host-checks-v1`,
`android-debug-v1`, and `android-release-v1` for publishing. Main runs warm the
verification caches for subsequent pull requests. Pull requests also save Rust
caches for their own subsequent runs. Gradle retains its default cache policy:
main writes the cache and pull requests restore it read-only.
