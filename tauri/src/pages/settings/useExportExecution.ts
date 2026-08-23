/**
 * P009：导出执行表单状态与 handleExport（自 useExportImportPage 拆出）。
 * 承载密码/提示词/保存路径/软警告流程 + Android content:// URI 中转导出；
 * 范围选择（useExportScope）与估算（useExportEstimate）仍由调用方组合后传入。
 */
import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import {
  cleanupStagedFile,
  copyStagedFileToDest,
  isUriPath,
  prepareStagedDownloadPath,
} from '@/lib/mobileFileTransfer';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { CloudTargetInfo } from '@/types/exportImport';

/** 导出范围快照（由 useExportScope + 标签/偏好开关组成，调用方保证引用稳定）。 */
export interface ExportScopeSnapshot {
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  selectedTags: Set<string>;
  includeAttachments: boolean;
  selectedAttachmentIds: Set<string>;
  includePreferences: boolean;
  includeBehavioral: boolean;
}

interface UseExportExecutionOptions {
  accountId: string;
  cloudTargets: CloudTargetInfo[];
  scope: ExportScopeSnapshot;
  totalSelected: number;
}

export function useExportExecution({
  accountId,
  cloudTargets,
  scope,
  totalSelected,
}: UseExportExecutionOptions) {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const [exportPassword, setExportPassword] = useState('');
  const [exportPasswordConfirm, setExportPasswordConfirm] = useState('');
  const [exportHint, setExportHint] = useState('');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [showHintWarning, setShowHintWarning] = useState(false);
  const skipHintCheckRef = useRef(false);
  const [showWeakPasswordWarning, setShowWeakPasswordWarning] = useState(false);
  const skipWeakPasswordCheckRef = useRef(false);

  // P034: 组件卸载时清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
  useEffect(() => {
    return () => {
      setExportPassword('');
      setExportPasswordConfirm('');
    };
    // setState 引用稳定，仅需挂载时注册一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Export handler
  const handleExport = async () => {
    if (totalSelected === 0 || !exportPassword || !savePath) return;

    if (exportPassword !== exportPasswordConfirm) {
      onError(new Error(t('settings:password_mismatch')), '');
      return;
    }

    // 检查 1: 密码提示词包含密码内容 → 软警告
    if (!skipHintCheckRef.current && exportHint && exportPassword.length >= 3) {
      const pwLower = exportPassword.toLowerCase();
      const hintLower = exportHint.toLowerCase();
      let hintContainsPassword = false;
      for (let i = 0; i <= pwLower.length - 3; i++) {
        if (hintLower.includes(pwLower.slice(i, i + 3))) {
          hintContainsPassword = true;
          break;
        }
      }
      if (hintContainsPassword) {
        setShowHintWarning(true);
        return;
      }
    }

    // 检查 2: 密码安全性低（不足 8 位）→ 软警告
    if (!skipWeakPasswordCheckRef.current && exportPassword.length < 8) {
      setShowWeakPasswordWarning(true);
      return;
    }

    setIsExporting(true);
    let stagedExportPath: string | null = null;
    try {
      let targetSavePath = savePath;
      // Android 保存对话框返回 content:// URI，Rust 无法直接写入，需要先写到缓存再中转
      if (savePath && isUriPath(savePath)) {
        stagedExportPath = await prepareStagedDownloadPath('solosoul_export.solosoul');
        targetSavePath = stagedExportPath;
      }

      const exportedPath = await invoke<string>('export_execute', {
        accountId: accountId,
        req: {
          scope: {
            selectedPageIds: Array.from(scope.selectedPageIds),
            selectedObjectIds: Array.from(scope.selectedObjectIds),
            selectedTags: Array.from(scope.selectedTags),
            includeAttachments: scope.includeAttachments,
            selectedAttachmentIds: Array.from(scope.selectedAttachmentIds),
            includePreferences: scope.includePreferences,
            includeBehavioral: scope.includeBehavioral,
          },
          password: exportPassword,
          passwordHint: exportHint || null,
          savePath: targetSavePath,
        },
      });

      if (stagedExportPath && savePath) {
        await copyStagedFileToDest(exportedPath, savePath);
      }

      // 导出成功后重置 skip ref，下次导出重新检查
      skipHintCheckRef.current = false;
      skipWeakPasswordCheckRef.current = false;
      // P034: 导出成功后立即清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
      setExportPassword('');
      setExportPasswordConfirm('');
      // B-04：导出目标位于云盘同步目录时，提示等待云盘客户端完成上传
      const cloudTargetName = cloudTargets.find((ct) =>
        targetSavePath.startsWith(ct.path),
      )?.name;
      onSuccess(
        cloudTargetName
          ? t('settings:export_success_cloud', { cloud: cloudTargetName })
          : t('settings:export_success'),
      );
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:export_failed'));
    } finally {
      if (stagedExportPath) {
        await cleanupStagedFile(stagedExportPath);
      }
      setIsExporting(false);
    }
  };

  return {
    exportPassword,
    setExportPassword,
    exportPasswordConfirm,
    setExportPasswordConfirm,
    exportHint,
    setExportHint,
    savePath,
    setSavePath,
    isExporting,
    showHintWarning,
    setShowHintWarning,
    showWeakPasswordWarning,
    setShowWeakPasswordWarning,
    skipHintCheckRef,
    skipWeakPasswordCheckRef,
    handleExport,
  };
}
