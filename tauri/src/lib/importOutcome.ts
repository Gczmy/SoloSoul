import type { TFunction } from 'i18next';
import type { ImportResult } from '@/types/exportImport';
import { resolveBackendErrorMessage } from './backendError';

/** 普通导入、云快照与账户恢复统一解释已提交结果，不把 IPC resolve 当成完整成功。 */
export function importOutcomeError(result: ImportResult, t: TFunction): string | null {
  if (result.status === 'complete') return null;
  if (
    result.status === 'notCommitted' &&
    (result.errorCode === 'PASSWORD_REQUIRED' ||
      result.errorCode === 'BAD_PASSWORD' ||
      result.errorCode === 'DECRYPT_FAILED')
  ) {
    return resolveBackendErrorMessage(`__IMPORT_ERR__:${result.errorCode}`);
  }
  const stage = t(`settings:import_stage_${result.failureStage ?? 'preparation'}`);
  return t(
    result.status === 'partial' ? 'settings:import_partial' : 'settings:import_not_committed',
    {
      stage,
      count: result.objectCount,
      attachments: result.attachmentCount,
      templates: result.templateCount,
      snapshots: result.snapshotCount,
      files: result.attachmentFilesWritten,
    },
  );
}
