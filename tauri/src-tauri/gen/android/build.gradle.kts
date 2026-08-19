buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:8.11.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

tasks.register("clean").configure {
    delete("build")
}

// ============================================================================
// SoloSoul 补丁：tauri-android 构建时补丁（移植 tauri upstream #15798）
// 修复 Activity 销毁重建后 ActivityResultLauncher 未重新注册，导致 SAF 目录选择器
// 等 startActivityForResult 的结果丢失、插件 invoke 永久挂起（安卓端选择保险库
// 外部目录一直"加载中"、无法进入下一步）。
//
// 背景：tauri-build 每次构建都会重写 tauri.settings.gradle，把 :tauri-android 指向
// cargo registry 里的模块源码（版本由 Cargo.lock 锁定），因此无法静态替换——改为在
// 任何 Kotlin/Java 编译前把 patches/ 下的补丁文件覆盖到模块源码目录。
//
// 升级 tauri 至含 #15798 的版本后，应删除本任务与 patches/tauri-android/ 目录。
// ============================================================================
gradle.projectsEvaluated {
    val tauriAndroid = rootProject.findProject(":tauri-android")
    if (tauriAndroid == null) {
        throw GradleException("[SoloSoul] 未找到 :tauri-android 模块（tauri.settings.gradle 是否已由 cargo 生成？）")
    }
    val moduleDir = tauriAndroid.projectDir
    val patchDir = rootProject.file("patches/tauri-android")
    val marker = "SoloSoul PATCH"

    val patchTask = rootProject.tasks.register("patchTauriAndroidPluginManager") {
        doLast {
            listOf("PluginManager.kt", "PluginHandle.kt").forEach { name ->
                val target = moduleDir.resolve("src/main/java/app/tauri/plugin/$name")
                val patch = patchDir.resolve(name)
                check(target.exists()) { "[SoloSoul] 未找到 tauri-android $name（tauri 版本路径变化？）: $target" }
                check(patch.exists()) { "[SoloSoul] 补丁文件缺失: $patch" }
                val content = target.readText()
                if (content.contains(marker)) {
                    // 已打补丁（幂等，避免二次构建重复覆盖时误报）
                    return@forEach
                }
                // 仅允许对已知的原版源码打补丁；tauri 升级后此处失败，要求人工复核
                val isOriginal = if (name == "PluginManager.kt") {
                    content.contains("if (::activity.isInitialized)")
                } else {
                    content.contains("val activity = manager.activity")
                }
                check(isOriginal) { "[SoloSoul] tauri-android $name 源码与补丁预期不符（tauri 升级？），请人工复核 SoloSoul 补丁（tauri #15798 移植）" }
                target.writeText(patch.readText())
                println("[SoloSoul] 已应用 tauri-android $name 补丁（#15798 移植：Activity 重建后 launcher 重注册）")
            }
        }
    }

    tauriAndroid.tasks.configureEach {
        if (name.startsWith("compile") && (name.contains("Kotlin") || name.contains("Java"))) {
            dependsOn(patchTask)
        }
    }
}

