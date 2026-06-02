// =============================================================================
// Attachment Task Models
// =============================================================================

/// 用于取消异步任务的简单令牌。
///
/// 在加密/解密等无法中途中断的阻塞操作前后检查此令牌，
/// 若已取消则清理已生成的文件并退出。
class CancelToken {
  bool _isCancelled = false;

  bool get isCancelled => _isCancelled;

  void cancel() => _isCancelled = true;
}

/// 附件任务的状态枚举。
enum TaskStatus {
  pending,    // 等待开始
  encrypting, // 加密中（阻塞，无法取消）
  writing,    // 写入磁盘中
  completed,  // 完成
  cancelled,  // 已取消
  error,      // 出错
}

/// 表示一个正在上传的附件任务。
///
/// 此为纯 UI 状态类，不持久化到数据模型中。
class UploadTask {
  final String tempId;
  final String fileName;
  final int size;
  final CancelToken cancelToken;
  double progress;
  TaskStatus status;
  String? errorMessage;

  UploadTask({
    required this.tempId,
    required this.fileName,
    required this.size,
    required this.cancelToken,
    this.progress = 0.0,
    this.status = TaskStatus.pending,
    this.errorMessage,
  });

  bool get isCancellable => status != TaskStatus.encrypting;
}

/// 表示一个正在下载的附件任务。
///
/// 此为纯 UI 状态类，不持久化到数据模型中。
class DownloadTask {
  final String attachmentId;
  final String fileName;
  final int size;
  final CancelToken cancelToken;
  double progress;
  TaskStatus status;
  String? errorMessage;

  DownloadTask({
    required this.attachmentId,
    required this.fileName,
    required this.size,
    required this.cancelToken,
    this.progress = 0.0,
    this.status = TaskStatus.pending,
    this.errorMessage,
  });

  bool get isCancellable => status != TaskStatus.encrypting;
}
