plugins {
    id("com.android.application")
    id("com.starter.easylauncher") version "6.4.1"
    id("io.gitlab.arturbosch.detekt") version "1.23.8"
}

val keystoreFile = providers.environmentVariable("ACT4G_KEYSTORE_FILE").orNull
val keystorePassword = providers.environmentVariable("ACT4G_KEYSTORE_PASSWORD").orNull
val keyAliasValue = providers.environmentVariable("ACT4G_KEY_ALIAS").orNull
val keyPasswordValue = providers.environmentVariable("ACT4G_KEY_PASSWORD").orNull
val signingReady = listOf(
    keystoreFile,
    keystorePassword,
    keyAliasValue,
    keyPasswordValue,
).all { !it.isNullOrBlank() }

android {
    namespace = "com.storytellerf.act4g"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.storytellerf.act4g"
        minSdk = 26
        targetSdk = 34
        versionCode = providers.environmentVariable("ACT4G_VERSION_CODE").orNull?.toInt() ?: 1
        versionName = providers.environmentVariable("ACT4G_VERSION_NAME").orNull ?: "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        manifestPlaceholders["nativeLibraryName"] = "act4g"
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
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
            keepDebugSymbols += listOf("*/arm64-v8a/libact4g.so")
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

easylauncher {
    buildTypes {
        register("debug") {
            enable(false)
        }
        register("alpha") {
            filters(
                chromeLike(
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

dependencies {
    implementation("androidx.core:core:1.19.0")
    implementation("androidx.core:core-splashscreen:1.2.0")
    implementation("androidx.biometric:biometric:1.1.0")
    implementation("androidx.media:media:1.8.0")
}
