import { Loader2 } from 'lucide-react';
import type { useTranslation } from 'react-i18next';

type T = ReturnType<typeof useTranslation>['t'];

/** 二维码数据未就绪时的状态占位：加载 spinner 或错误文本（固定高度，防卡片高度突变）。 */
export function QrStatusBlock({
  loading,
  error,
  t,
}: {
  loading: boolean;
  error: string | null;
  t: T;
}) {
  if (loading) {
    return (
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          minHeight: 360,
        }}
      >
        <Loader2 size={32} style={{ animation: 'spin 1s linear infinite' }} />
        <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
          {t('common:loading')}
        </span>
      </div>
    );
  }
  if (error) {
    return (
      <div
        style={{
          color: '#e74c3c',
          fontSize: 'var(--text-body-sm)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: 360,
        }}
      >
        {error}
      </div>
    );
  }
  return null;
}
