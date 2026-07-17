# Android OCR 插件生命周期优化 Spike（MOB-P4-02）

> **状态**：已完成优化并落地到 `MobileOcrPlugin.kt` 与 `mobile_ocr_plugin.rs`。  
> **作者**：AI 编程助手  
> **日期**：2026-07-17

---

## 1. 背景与问题

SoloSoul Android 端使用 **ML Kit Text Recognition v2** 进行本地 OCR。早期实现中，每次调用 `scanImage` 都会新建一个 `TextRecognizer` 实例：

```kotlin
val recognizer = TextRecognition.getClient(
    ChineseTextRecognizerOptions.Builder().build()
)
recognizer.process(image).addOnSuccessListener { ... }
```

这带来两个问题：

1. **性能损耗**：ML Kit 的 `TextRecognizer` 内部包含模型加载与管线初始化，频繁创建/销毁会增加 GC 压力和首次识别延迟。
2. **资源泄漏**：`TextRecognizer` 持有原生资源，虽然 ML Kit 内部有最终化逻辑，但显式 `close()` 更符合 Tauri 插件生命周期管理要求。

---

## 2. 优化方案

### 2.1 复用 Recognizer

将 `TextRecognizer` 提升为插件成员变量，在插件构造/首次使用时初始化，后续请求复用同一实例：

```kotlin
private val recognizer: TextRecognizer by lazy {
    TextRecognition.getClient(ChineseTextRecognizerOptions.Builder().build())
}
```

使用 `by lazy` 的好处：
- 插件创建时不立即加载模型，避免拖慢 Activity 启动。
- 首次扫描时懒加载，之后复用。

### 2.2 生命周期关闭

Tauri v2 的 Kotlin 插件基类提供 `onDestroy()` 回调。重写该方法，在插件销毁时关闭 recognizer：

```kotlin
override fun onDestroy() {
    super.onDestroy()
    if (::recognizer.isInitialized) {
        recognizer.close()
    }
}
```

### 2.3 线程与并发

ML Kit 的 `process(image)` 是异步 API，内部已走 ML Kit 线程池。Rust 端通过 `tokio::task::spawn_blocking` 将调用放入阻塞线程池，避免占用 tokio runtime。

---

## 3. 验收标准

- [x] `MobileOcrPlugin.kt` 不再每次新建 `TextRecognizer`。
- [x] 插件销毁时显式 `close()` recognizer。
- [x] 桌面端 `mobile_ocr_plugin.rs` 行为不变（仍为占位实现）。
- [x] 移动 target 编译通过。

---

## 4. 后续可改进点

1. **模型预热**：在应用启动空闲时预加载 recognizer，减少首次扫描延迟。
2. **多语言 recognizer 切换**：当前固定中文 recognizer；若后续支持多语言，可改为根据参数动态选择并缓存多个实例。
3. **批量识别**：ML Kit 支持连续帧识别，可考虑为摄像头实时扫描场景提供 `startContinuousScan` / `stopContinuousScan` API。

---

## 5. 相关文件

- `tauri/src-tauri/gen/android/app/src/main/java/com/solosoul/app/MobileOcrPlugin.kt`
- `tauri/src-tauri/src/mobile_ocr_plugin.rs`
- `tauri/src/pages/scan/OcrPage.tsx`
