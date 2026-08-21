import { useTranslation } from 'react-i18next';
import { Sparkles } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { maskValue } from '@/lib/masking';

/** 字段推荐条目（对应后端 object_field_suggestions 返回，camelCase）。 */
export interface FieldSuggestion {
  objectId: string;
  objectName: string;
  fieldKey: string;
  fieldName: string;
  sensitivityLevel: string;
  value: string;
}

interface FieldSuggestionsProps {
  /** 当前字段的显示名（与其它对象 `__fields` 中的字段名匹配）。 */
  fieldName: string;
  /** 后端已按字段名过滤的推荐条目（未命中传空数组）。 */
  suggestions: FieldSuggestion[];
  /** 点击推荐项时回填原始值。 */
  onPick: (value: string) => void;
  /** 最多展示条数（超出折叠为「还有 N 条」）。 */
  limit?: number;
}

/** 公开内容单行展示限长（遮掩内容恒为 8 圆点，无需截断）。 */
const PUBLIC_VALUE_DISPLAY_LIMIT = 80;

/**
 * 字段推荐列表：编辑对象时，若其他对象存在同名字段且有内容，在字段下方展示
 * `[对象名][敏感度徽章][内容]`，其中内容按该字段在来源对象中的敏感度等级遮掩
 * （复用 lib/masking 的统一 8 圆点占位符；仅 public 明文展示并截断）。点击条目
 * 将真实值回填到当前字段。
 */
export function FieldSuggestions({
  fieldName,
  suggestions,
  onPick,
  limit = 5,
}: FieldSuggestionsProps) {
  const { t } = useTranslation('editor');
  if (!suggestions || suggestions.length === 0) return null;

  const shown = suggestions.slice(0, limit);
  const hidden = suggestions.length - shown.length;

  return (
    <div data-testid="field-suggestions" style={{ marginTop: 6 }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 4,
          marginBottom: 4,
          fontSize: 'var(--text-badge)',
          color: 'var(--text-tertiary)',
        }}
      >
        <Sparkles size={12} style={{ flexShrink: 0 }} />
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {t('field_suggestions_label', {
            field: fieldName,
            defaultValue: `"${fieldName}" from other objects`,
          })}
        </span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {shown.map((s) => {
          const level = (s.sensitivityLevel as SensitivityLevel) || 'internal';
          const displayValue =
            level === 'public' && s.value.length > PUBLIC_VALUE_DISPLAY_LIMIT
              ? `${s.value.slice(0, PUBLIC_VALUE_DISPLAY_LIMIT)}…`
              : maskValue(s.value, level);
          return (
            <button
              key={`${s.objectId}::${s.fieldKey}`}
              type="button"
              data-testid="field-suggestion-item"
              title={t('field_suggestions_pick', {
                object: s.objectName,
                defaultValue: `Fill with value from "${s.objectName}"`,
              })}
              onClick={() => onPick(s.value)}
              className="interactive-toolbar"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '6px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 'var(--text-caption)',
                cursor: 'pointer',
                width: '100%',
                textAlign: 'left',
              }}
            >
              <span
                style={{
                  flexShrink: 0,
                  maxWidth: '40%',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  fontWeight: 600,
                  color: 'var(--text-secondary)',
                }}
              >
                {s.objectName}
              </span>
              <SensitivityBadge level={level} showText={false} />
              <span
                style={{
                  flex: 1,
                  minWidth: 0,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  color: 'var(--text-tertiary)',
                  fontVariantNumeric: 'tabular-nums',
                }}
              >
                {displayValue}
              </span>
            </button>
          );
        })}
        {hidden > 0 && (
          <div
            style={{
              padding: '2px 10px',
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
            }}
          >
            {t('field_suggestions_more', { n: hidden, defaultValue: `+${hidden} more` })}
          </div>
        )}
      </div>
    </div>
  );
}
