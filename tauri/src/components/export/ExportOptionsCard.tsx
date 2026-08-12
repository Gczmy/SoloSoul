import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { AttachmentLimitsInfo } from './AttachmentLimitsInfo';

/**
 * ExportSection 的「导出选项」卡片（P046 拆分：展示子组件）。
 * 包含附件/偏好设置/行为数据三个勾选项。
 */
export function ExportOptionsCard({
  includeAttachments,
  includePreferences,
  includeBehavioral,
  onSetIncludeAttachments,
  onSetIncludePreferences,
  onSetIncludeBehavioral,
  t,
}: {
  includeAttachments: boolean;
  includePreferences: boolean;
  includeBehavioral: boolean;
  onSetIncludeAttachments: (v: boolean) => void;
  onSetIncludePreferences: (v: boolean) => void;
  onSetIncludeBehavioral: (v: boolean) => void;
  t: TFunction;
}) {
  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
        {t('settings:export_options')}
      </h3>
      <div style={{ padding: '4px 0' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              cursor: 'pointer',
              fontSize: 'var(--text-body-sm)',
            }}
          >
            <SelectCheckbox
              checked={includeAttachments}
              onChange={(v) => onSetIncludeAttachments(v)}
            />
            {t('settings:include_attachments')}
          </label>
          <AttachmentLimitsInfo />
        </div>
        <div
          style={{
            paddingLeft: 24,
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            marginTop: 2,
          }}
        >
          {t('settings:include_attachments_desc')}
        </div>
      </div>
      <div style={{ padding: '4px 0' }}>
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
          }}
        >
          <SelectCheckbox
            checked={includePreferences}
            onChange={(v) => onSetIncludePreferences(v)}
          />
          {t('settings:include_preferences')}
        </label>
        <div
          style={{
            paddingLeft: 24,
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            marginTop: 2,
          }}
        >
          {t('settings:include_preferences_desc')}
        </div>
      </div>
      <div style={{ padding: '4px 0' }}>
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
          }}
        >
          <SelectCheckbox
            checked={includeBehavioral}
            onChange={(v) => onSetIncludeBehavioral(v)}
          />
          {t('settings:include_behavioral')}
        </label>
        <div
          style={{
            paddingLeft: 24,
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            marginTop: 2,
          }}
        >
          {t('settings:include_behavioral_desc')}
        </div>
      </div>
    </Card>
  );
}
