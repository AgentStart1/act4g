import java.util.Properties

plugins {
    id("com.android.application")
    id("com.starter.easylauncher") version "6.4.1"
    id("io.gitlab.arturbosch.detekt") version "1.23.8"
}

val keystoreFile = providers.environmentVariable("GACT_KEYSTORE_FILE").orNull
val keystorePassword = providers.environmentVariable("GACT_KEYSTORE_PASSWORD").orNull
val keyAliasValue = providers.environmentVariable("GACT_KEY_ALIAS").orNull
val keyPasswordValue = providers.environmentVariable("GACT_KEY_PASSWORD").orNull
val signingReady = listOf(
    keystoreFile,
    keystorePassword,
    keyAliasValue,
    keyPasswordValue,
).all { !it.isNullOrBlank() }

android {
    namespace = "com.storytellerf.gact"
    compileSdk = 37
    ndkVersion = providers.gradleProperty("gact.ndkVersion").get()

    defaultConfig {
        applicationId = "com.storytellerf.gact"
        minSdk = 26
        targetSdk = 34
        versionCode = providers.environmentVariable("GACT_VERSION_CODE").orNull?.toInt() ?: 1
        versionName = providers.environmentVariable("GACT_VERSION_NAME").orNull ?: "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        manifestPlaceholders["nativeLibraryName"] = "gact"
    }

    signingConfigs {
        if (signingReady) {
            create("sharedRelease") {
                storeFile = file(keystoreFile!!)
                storePassword = keystorePassword
                keyAlias = keyAliasValue
                keyPassword = keyPasswordValue
            }
        }
    }

    buildTypes {
        val release = getByName("release") {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            if (signingReady) {
                signingConfig = signingConfigs.getByName("sharedRelease")
            }
        }
        create("alpha") {
            initWith(release)
            matchingFallbacks += "release"
        }
        getByName("debug") {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            jniLibs.directories += "src/main/jniLibs"
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
            keepDebugSymbols += listOf("*/arm64-v8a/libgact.so")
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

val resolvedNdkDirectory = androidComponents.sdkComponents.ndkDirectory
val expectedNdkVersion = providers.gradleProperty("gact.ndkVersion")
val ndkEnvironment = listOf("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME")
    .associateWith { providers.environmentVariable(it) }

tasks.register("verifyAndroidNdk") {
    group = "verification"
    description = "Checks that Gradle and the native build use the same NDK installation."
    doLast {
        val ndkDirectory = resolvedNdkDirectory.get().asFile.canonicalFile
        val properties = Properties().apply {
            ndkDirectory.resolve("source.properties").inputStream().use { load(it) }
        }
        check(properties.getProperty("Pkg.Revision") == expectedNdkVersion.get()) {
            "Gradle resolved an unexpected NDK: $ndkDirectory"
        }
        ndkEnvironment.forEach { (name, value) ->
            check(file(value.get()).canonicalFile == ndkDirectory) {
                "$name does not match Gradle's NDK: $ndkDirectory"
            }
        }
        logger.lifecycle("Gradle NDK ${expectedNdkVersion.get()}: $ndkDirectory")
    }
}

easylauncher {
    buildTypes {
        register("debug") {
            filters(
                customRibbon(
                    label = "DEBUG",
                    ribbonColor = "#238636",
                    labelColor = "#FFFFFF",
                )
            )
        }
        register("alpha") {
            filters(
                customRibbon(
                    label = "ALPHA",
                    ribbonColor = "#D29922",
                    labelColor = "#FFFFFF",
                )
            )
        }
        register("release") {
            enable(false)
        }
    }
}

detekt {
    buildUponDefaultConfig = true
    config.setFrom(rootProject.files("config/detekt/detekt.yml"))
}

tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach {
    setSource(
        rootProject.fileTree(rootProject.projectDir) {
            include("**/*.kt", "**/*.kts")
            exclude("**/.gradle/**", "**/build/**")
        }
    )
    reports {
        html.required.set(true)
        sarif.required.set(true)
        xml.required.set(true)
        txt.required.set(false)
        md.required.set(false)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.compilerArgs.add("-Xlint:deprecation")
}

dependencies {
    implementation("androidx.core:core:1.19.1")
    implementation("androidx.core:core-splashscreen:1.2.0")
    implementation("androidx.biometric:biometric:1.1.0")
    implementation("androidx.media:media:1.8.0")
}
