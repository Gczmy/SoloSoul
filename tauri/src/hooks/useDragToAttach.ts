import { useState, useEffect, useRef } from 'react';
import { uploadAttachmentsSequentially, filterOutDirectories } from '@/lib/attachmentUpload';
import { useUiStore } from '@/stores/uiStore';
import i18n from '@/lib/i18n';

export interface DragUploadState {
  /** 文件正拖拽到目标区域上方 */
  isDraggingOver: boolean;
  /** 正在上传中 */
  isUploading: boolean;
  /** 当前上传到第几个 (从 0 开始) */
  currentIndex: number;
  /** 当前批次的文件总数 */
  totalFiles: number;
  /** 当前正在上传的文件名 */
  currentFileName: string;
  /** 队列中等待的文件数量 */
  pendingFiles: number;
}

const initialState: DragUploadState = {
  isDraggingOver: false,
  isUploading: false,
  currentIndex: 0,
  totalFiles: 0,
  currentFileName: '',
  pendingFiles: 0,
};

interface UseDragToAttachOptions {
  /** 所有文件上传完成后的回调，用于刷新列表 */
  onComplete?: () => void;
}

/** 从 Tauri Event<DragDropEvent> 中提取统一的事件字段 */
interface DragDropPayload {
  type: 'enter' | 'over' | 'drop' | 'leave';
  paths?: string[];
  position?: { x: number; y: number };
}

/**
 * 注册窗口级 drag-drop 监听器。
 * 返回 unlisten 函数或 null（不可用时）。
 */
async function registerDragDropListener(
  handler: (payload: DragDropPayload) => void,
): Promise<(() => void) | null> {
  try {
    const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const win = getCurrentWebviewWindow();
    return await win.onDragDropEvent((event) => {
      handler(event.payload as DragDropPayload);
    });
  } catch {
    return null;
  }
}



/**
 * useDragToAttach
 *
 * 为指定对象提供文件拖拽上传能力。
 * 基于 Tauri v2 原生的 `getCurrentWebviewWindow().onDragDropEvent()`，
 * 无需安装额外插件。
 *
 * @param objectId - 目标对象的 ID，拖入的文件将附加到此对象
 * @param options  - 可选回调
 * @returns dragRef - 绑定到目标元素的 ref
 *          dragState - 当前拖拽/上传状态，用于渲染 UI 反馈
 */
export function useDragToAttach(
  objectId: string | null,
  options?: UseDragToAttachOptions,
) {
  const ref = useRef<HTMLDivElement>(null);
  const objectIdRef = useRef(objectId);
  const onCompleteRef = useRef(options?.onComplete);
  objectIdRef.current = objectId;
  onCompleteRef.current = options?.onComplete;

  const [dragState, setDragState] = useState<DragUploadState>(initialState);
  const isUploadingRef = useRef(false);
  /** 排队等待上传的批次队列，每个元素是一组文件路径 */
  const pendingQueueRef = useRef<string[][]>([]);
  /**
   * 用于去重的最近 drop 签名（路径列表的 JSON 字符串）。
   * Tauri v2 的 onDragDropEvent 在某些场景下会触发重复 drop 事件。
   * 保存最近 1 秒内的签名以避免重复处理。
   */
  const recentDropSignaturesRef = useRef<{ sig: string; timer: ReturnType<typeof setTimeout> }[]>([]);

  /** 检查并记录 drop 签名，重复时返回 true */
  const isDuplicateDrop = useRef((paths: string[]): boolean => {
    const sig = JSON.stringify([...paths].sort());
    if (recentDropSignaturesRef.current.some((entry) => entry.sig === sig)) {
      return true;
    }
    const timer = setTimeout(() => {
      recentDropSignaturesRef.current = recentDropSignaturesRef.current.filter(
        (e) => e.timer !== timer,
      );
    }, 1000);
    recentDropSignaturesRef.current.push({ sig, timer });
    return false;
  }).current;

  /** 处理一个批次的上传（包含队列调度） */
  const processBatch = useRef(async (paths: string[], objId: string) => {
    const runningObjId = objId;
    try {
      await uploadAttachmentsSequentially(
        paths,
        runningObjId,
        (i, total, fileName) => {
          if (mountedRef.current) {
            setDragState((prev) => ({
              ...prev,
              currentIndex: i,
              currentFileName: fileName,
            }));
          }
        },
      );
    } catch (e) {
      // 保留 console.error 用于调试追溯（错误对象 e 仅在 catch 作用域内）
      // eslint-disable-next-line no-console
      console.error('Drag-drop upload failed:', e);
      useUiStore.getState().showToast({
        type: 'error',
        message: i18n.t('upload_failed'),
      });
    } finally {
      // 检查队列中是否有下一个批次
      const nextBatch = pendingQueueRef.current.shift();
      if (nextBatch && mountedRef.current) {
        // 更新 pendingFiles 计数
        const remaining = pendingQueueRef.current.reduce(
          (sum, batch) => sum + batch.length,
          0,
        );
        setDragState({
          isDraggingOver: false,
          isUploading: true,
          currentIndex: 0,
          totalFiles: nextBatch.length,
          currentFileName:
            nextBatch[0]?.split('/').pop() ||
            nextBatch[0]?.split('\\').pop() ||
            '',
          pendingFiles: remaining,
        });
        await processBatchRef.current(nextBatch, runningObjId);
      } else {
        isUploadingRef.current = false;
        if (mountedRef.current) {
          setDragState(initialState);
          onCompleteRef.current?.();
        }
      }
    }
  }).current;

  // 用 ref 保存 processBatch 以便在闭包中引用最新的版本
  const processBatchRef = useRef(processBatch);

  // 用 ref 保存 mounted 状态（被多个异步闭包共享）
  const mountedRef = useRef(true);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    mountedRef.current = true;

    (async () => {
      unlisten = await registerDragDropListener((payload) => {
        if (!mountedRef.current) return;

        const el = ref.current;
        const currentObjectId = objectIdRef.current;
        if (!el || !currentObjectId) return;

        const pos = payload.position;

        // ── 视觉层级检测：使用 elementFromPoint 确认当前组件是视觉顶层接收者 ──
        // 由于 useDragToAttach 注册的是窗口级全局监听器，所有实例都会收到每次 drop 事件。
        // 当弹窗（如 AttachmentViewer/ObjectDetailModal）覆盖在工作区卡片上方时，
        // 被覆盖的卡片虽然 getBoundingClientRect() 仍然有效，但实际上不应接收 drop。
        // elementFromPoint(x, y) 返回视觉上最顶层的元素，检查它是否属于当前 ref 的子树。
        const topEl = pos !== undefined
          ? document.elementFromPoint(pos.x, pos.y)
          : null;
        const isTopReceiver = topEl !== null && (topEl === el || el.contains(topEl));

        const rect = el.getBoundingClientRect();
        const isOverBounds = isTopReceiver &&
          pos !== undefined &&
          pos.x >= rect.left &&
          pos.x <= rect.right &&
          pos.y >= rect.top &&
          pos.y <= rect.bottom;

        switch (payload.type) {
          case 'enter':
          case 'over':
            setDragState((prev) => ({ ...prev, isDraggingOver: !!isOverBounds }));
            break;
          case 'leave':
            setDragState((prev) => ({ ...prev, isDraggingOver: false }));
            break;
          case 'drop': {
            const rawPaths = payload.paths;
            if (!rawPaths || rawPaths.length === 0) break;
            if (!isOverBounds) break;
            // 去重：Tauri v2 onDragDropEvent 可能触发重复 drop 事件
            if (isDuplicateDrop(rawPaths)) break;

            // ── 过滤目录：拖拽的文件夹无法上传，提示用户 ──
            const doFilter = async () => {
              const pathsToDrop = rawPaths;
              const { files, dirs } = await filterOutDirectories(pathsToDrop);

              if (dirs.length > 0) {
                useUiStore.getState().showToast({
                  type: 'warning',
                  message: i18n.t('folder_drop_not_supported', { n: dirs.length }),
                });
              }

              if (files.length === 0) {
                // 全是文件夹，重置拖拽状态用户界面
                setDragState(initialState);
                return;
              }

              if (isUploadingRef.current) {
                // ── 正在上传中，将文件批次加入队列 ──
                pendingQueueRef.current.push(files);
                const totalPending = pendingQueueRef.current.reduce(
                  (sum, batch) => sum + batch.length,
                  0,
                );
                setDragState((prev) => ({ ...prev, pendingFiles: totalPending }));
                return;
              }

              // ── 空闲，立即开始上传 ──
              setDragState((prev) => ({ ...prev, isDraggingOver: false }));
              isUploadingRef.current = true;

              const objId = currentObjectId;
              const firstFileName =
                files[0]?.split('/').pop() ||
                files[0]?.split('\\').pop() ||
                '';

              setDragState({
                isDraggingOver: false,
                isUploading: true,
                currentIndex: 0,
                totalFiles: files.length,
                currentFileName: firstFileName,
                pendingFiles: 0,
              });

              processBatchRef.current(files, objId);
            };

            doFilter();
            break;
          }
        }
      });
    })();

    return () => {
      mountedRef.current = false;
      pendingQueueRef.current = []; // 卸载时丢弃队列
      // 清理去重定时器
      for (const entry of recentDropSignaturesRef.current) {
        clearTimeout(entry.timer);
      }
      recentDropSignaturesRef.current = [];
      unlisten?.();
    };
    // 只在 mount/unmount 时注册/注销监听器
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { ref, dragState };
}
