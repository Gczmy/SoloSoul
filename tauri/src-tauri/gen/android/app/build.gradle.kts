import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "com.solosoul.app"
    // universal flavor 的 abiFilters 由 buildSrc RustPlugin 按 abiList 属性生成，
    // 且可能被命令行 -P 覆盖（tauri CLI 构建全量 target 时），这里显式钉死，
    // 保证任何构建入口下 universal 包都不含模拟器专用的 x86/x86_64。
    productFlavors {
        getByName("universal") {
            ndk {
                abiFilters.clear()
                abiFilters += listOf("arm64-v8a", "armeabi-v7a")
            }
        }
    }
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.solosoul.app"
        minSdk = 28
        targetSdk = 36
        // versionCode = 基础值（tauri.properties）+ CI 构建序号，保证每次发布单调递增
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt() +
            (System.getenv("GITHUB_RUN_NUMBER") ?: "0").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")

        // 只打包 arm64 与 armv7，覆盖绝大多数 Android 真机；
        // x86/x86_64 仅用于模拟器调试，release universal 包不再携带。
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }
    signingConfigs {
        create("release") {
            val path = System.getenv("SOLOSOUL_KEYSTORE_PATH")
            if (path != null) {
                storeFile = file(path)
                storePassword = System.getenv("SOLOSOUL_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("SOLOSOUL_KEY_ALIAS") ?: "solosoul-upload"
                keyPassword = System.getenv("SOLOSOUL_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            isShrinkResources = true
            isCrunchPngs = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
            // release 构建保留 native debug symbols 用于 Play Console 崩溃还原，
            // 但符号不打包进 APK，保持 APK 体积最小；同时禁用 legacy packaging，
            // 让 .so 在 APK 中 page-aligned，安装时无需解压，减少安装后占用。
            ndk {
                debugSymbolLevel = "FULL"
            }
            packaging {
                jniLibs.useLegacyPackaging = false
                resources {
                    excludes += setOf(
                        "META-INF/**",
                        "META-INF/NOTICE",
                        "META-INF/LICENSE",
                        "META-INF/DEPENDENCIES",
                        "META-INF/AL2.0",
                        "META-INF/LGPL2.1",
                    )
                }
            }
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

// 安卓端不使用 ONNX 模型（OCR 走 ML Kit，本地 Embedding 移动端不支持，
// ocr_install_bundled_model 移动端返回不支持），但 Tauri 会把 src-tauri/resources
// 全量复制进 assets（含 ~55MB models）。packagingOptions 的 excludes 对 assets
// 不生效，因此在 assets 合并任务后直接从中间产物删除该目录。
afterEvaluate {
    tasks
        .matching { it.name.matches(Regex("merge.+Assets")) && !it.name.contains("Test") }
        .configureEach {
            val taskName = name
            doLast {
                val variant = taskName.removePrefix("merge").removeSuffix("Assets")
                delete(layout.buildDirectory.dir("intermediates/assets/$variant/$taskName/models"))
            }
        }
}

dependencies {
    implementation("androidx.core:core-ktx:1.16.0")
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    implementation("com.google.mlkit:text-recognition-chinese:16.0.1")
    implementation("androidx.biometric:biometric:1.1.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")