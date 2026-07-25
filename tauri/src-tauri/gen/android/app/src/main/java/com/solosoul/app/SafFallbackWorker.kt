package com.solosoul.app

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * WorkManager 兜底同步 Worker。
 *
 * 在应用被系统回收后，由 WorkManager 周期调度，将本地 Vault 缓存目录
 * 同步到用户选择的 SAF tree URI。同步期间使用 [SafSyncHelper] 的文件锁，
 * 避免与应用前台同步并发。
 *
 * 该 Worker 不显示前台服务通知，仅在满足系统约束（默认非低电量）时静默执行。
 */
class SafFallbackWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {

    companion object {
        private const val UNIQUE_WORK_NAME = "com.solosoul.SafFallbackWorker"
        private const val PREFS_NAME = "saf_fallback_sync_prefs"
        private const val KEY_LOCAL_DIR = "local_dir"
        private const val KEY_TREE_URI = "tree_uri"

        /**
         * 保存后台同步所需的本地目录与 SAF tree URI。
         */
        fun saveSyncConfig(context: Context, localDir: String, treeUri: String) {
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .putString(KEY_LOCAL_DIR, localDir)
                .putString(KEY_TREE_URI, treeUri)
                .apply()
        }

        /**
         * 清除已保存的同步配置。
         */
        fun clearSyncConfig(context: Context) {
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .clear()
                .apply()
        }

        /**
         * 读取已保存的同步配置，若任一为空则返回 null。
         */
        fun readSyncConfig(context: Context): Pair<String, String>? {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            val localDir = prefs.getString(KEY_LOCAL_DIR, null)
            val treeUri = prefs.getString(KEY_TREE_URI, null)
            return if (localDir != null && treeUri != null) {
                localDir to treeUri
            } else {
                null
            }
        }

        /**
         * 调度周期性的 SAF 后台同步任务。
         *
         * 默认间隔 15 分钟（WorkManager 允许的最小周期），约束为不在低电量时执行。
         * 使用 [ExistingPeriodicWorkPolicy.UPDATE] 以便在配置变化时更新现有任务。
         */
        fun schedule(context: Context, localDir: String, treeUri: String) {
            saveSyncConfig(context, localDir, treeUri)

            val constraints = Constraints.Builder()
                .setRequiresBatteryNotLow(true)
                .build()

            val request = PeriodicWorkRequestBuilder<SafFallbackWorker>(15, TimeUnit.MINUTES)
                .setConstraints(constraints)
                .addTag(UNIQUE_WORK_NAME)
                .build()

            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                UNIQUE_WORK_NAME,
                ExistingPeriodicWorkPolicy.UPDATE,
                request
            )
        }

        /**
         * 取消已调度的 SAF 后台同步任务。
         */
        fun cancel(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(UNIQUE_WORK_NAME)
            clearSyncConfig(context)
        }
    }

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val (localDir, treeUri) = readSyncConfig(applicationContext) ?: return@withContext Result.failure()
        val localDirFile = File(localDir)
        if (!localDirFile.exists() || !localDirFile.isDirectory) {
            android.util.Log.w("SoloSoul", "SafFallbackWorker: local dir does not exist, skipping sync")
            return@withContext Result.failure()
        }

        android.util.Log.i("SoloSoul", "SafFallbackWorker: starting background SAF sync")
        return@withContext try {
            val result = SafSyncHelper.syncLocalDirToTree(applicationContext, localDirFile, treeUri) { fileName ->
                android.util.Log.d("SoloSoul", "SafFallbackWorker: synced $fileName")
            }
            if (result.isSuccess) {
                android.util.Log.i("SoloSoul", "SafFallbackWorker: background SAF sync completed")
                Result.success()
            } else {
                val ex = result.exceptionOrNull()
                android.util.Log.e("SoloSoul", "SafFallbackWorker: sync failed: ${ex?.message}", ex)
                Result.retry()
            }
        } catch (e: Exception) {
            android.util.Log.e("SoloSoul", "SafFallbackWorker: unexpected error", e)
            Result.retry()
        }
    }
}
