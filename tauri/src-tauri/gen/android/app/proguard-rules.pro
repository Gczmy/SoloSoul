# SoloSoul ProGuard/R8 规则
# ============================================================

# ── Tauri / WebView 桥接层 ──
# Tauri 通过 Java/Kotlin 接口与 WebView 通信，这些接口不能被混淆或删除。
-keep class app.tauri.** { *; }
-keep class com.solosoul.app.** { *; }
-keepclassmembers class * {
    @app.tauri.annotation.Command <methods>;
    @app.tauri.annotation.InvokeArg <fields>;
}

# ── Serde / JSON 序列化 ──
# Rust 侧通过 serde_json 序列化/反序列化的数据类需要保留无参构造函数与字段。
-keepclassmembers class * {
    *** Companion;
}
-keepclasseswithmembers class * {
    kotlinx.serialization.Serializable <fields>;
}

# ── ML Kit 文字识别 ──
# ML Kit 与 Play Services 自带 consumer ProGuard 规则和 @Keep 注解，
# 保留其自身 keep 即可；过度 keep 会阻止 R8 优化，显著增加 APK 体积。
-dontwarn com.google.mlkit.**
-dontwarn com.google.android.gms.**

# ── AndroidX / Material ──
# Tauri 插件桥接层直接引用 AndroidX 类（不通过反射），app.tauri.** 的 keep 已覆盖。
# 此处不做全局 keep 以允许 R8 优化/移除未使用的 AndroidX 类。
-dontwarn androidx.**
-dontwarn com.google.android.material.**

# ── 崩溃堆栈可读性 ──
-keepattributes SourceFile,LineNumberTable
-keepattributes *Annotation*, InnerClasses, Signature, EnclosingMethod
-renamesourcefileattribute SourceFile

# ── Kotlin 协程 ──
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}

# ── 通用优化 ──
# 保留枚举的 values()/valueOf() 方法
-keepclassmembers enum * {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}
# 保留 Parcelable 实现
-keepclassmembers class * implements android.os.Parcelable {
    public static final android.os.Parcelable$Creator CREATOR;
}