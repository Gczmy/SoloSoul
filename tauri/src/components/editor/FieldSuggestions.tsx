import { useTranslation } from 'react-i18next';
import { ArrowDownLeft, Eye, EyeOff, Sparkles } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { maskValue } from '@/lib/masking';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useAuthStore } from '@/stores/authStore';
import { useSuggestionReveal, suggestionItemId } from './useSuggestionReveal';

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
  /** 点击「填入」按钮时回填原始值。 */
  onPick: (value: string) => void;
  /** 最多展示条数（超出折叠为「还有 N 条」）。 */
  limit?: number;
}

/** 公开内容单行展示限长（遮掩内容恒为 8 圆点，无需截断）。 */
const PUBLIC_VALUE_DISPLAY_LIMIT = 80;

/** 明文展示截断（public 与已揭示的非 public 统一限长）。 */
function truncateForDisplay(value: string): string {
  return value.length > PUBLIC_VALUE_DISPLAY_LIMIT
    ? `${value.slice(0, PUBLIC_VALUE_DISPLAY_LIMIT)}…`
    : value;
}

/**
 * 字段推荐列表：编辑对象时，若其他对象存在同名字段且有内容，在字段下方展示
 * `[对象名][敏感度徽章][内容]`，其中内容按该字段在来源对象中的敏感度等级遮掩
 * （复用 lib/masking 的统一 8 圆点占位符；仅 public 明文展示并截断）。
 *
 * 按敏感度分级展示/揭示：
 * - public / internal：始终明文展示（内部级在推荐场景与公开同权）；
 * - sensitive：掩码，点击条目切换揭示/隐藏（1 分钟 TTL 自动重掩）；
 * - critical：掩码，点击弹出主密码验证框（支持密码/PIN/生物识别），验证成功后才揭示。
 *
 * 每行右侧「填入」按钮将真实值回填到当前字段（不要求先揭示）。
 */
export function FieldSuggestions({
  fieldName,
  suggestions,
  onPick,
  limit = 5,
}: FieldSuggestionsProps) {
  const { t } = useTranslation(['editor', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    isRevealed,
    handleItemClick,
    showPwDialog,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    passwordHint,
    bioAvailable,
    handleBiometricUnlock,
  } = useSuggestionReveal(accountId);

  if (!suggestions || suggestions.length === 0) return null;

  const shown = suggestions.slice(0, limit);
  const hidden = suggestions.length - shown.length;

  return (
    <>
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
            // 公开/内部直接明文；敏感/关键掩码（按需揭示）
            const needsReveal = level === 'sensitive' || level === 'critical';
            const itemId = suggestionItemId(s);
            const revealed = isRevealed(itemId);
            const displayValue =
              !needsReveal || revealed ? truncateForDisplay(s.value) : maskValue(s.value, level);
            const rowTitle = needsReveal
              ? revealed
                ? t('field_suggestions_hide', { defaultValue: 'Click to hide' })
                : level === 'critical'
                  ? t('field_suggestions_critical_reveal', {
                      defaultValue: 'Verify master password to view',
                    })
                  : t('field_suggestions_reveal', {
                      defaultValue: 'Click to view plaintext',
                    })
              : undefined;
            return (
              <div
                key={itemId}
                className="interactive-toolbar"
                style={{
                  display: 'flex',
                  alignItems: 'stretch',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  color: 'var(--text-primary)',
                  overflow: 'hidden',
                }}
              >
                <button
                  type="button"
                  data-testid="field-suggestion-item"
                  title={rowTitle}
                  disabled={!needsReveal}
                  onClick={() => handleItemClick(s)}
                  style={{
                    flex: 1,
                    minWidth: 0,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '6px 10px',
                    border: 'none',
                    background: 'transparent',
                    color: 'var(--text-primary)',
                    fontSize: 'var(--text-caption)',
                    cursor: needsReveal ? 'pointer' : 'default',
                    textAlign: 'left',
                  }}
                >
                  <span
                    style={{
                      flexShrink: 0,
                      maxWidth: '32%',
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
                  {needsReveal && (
                    <span
                      style={{ flexShrink: 0, color: 'var(--text-tertiary)', display: 'flex' }}
                    >
                      {revealed ? <EyeOff size={13} /> : <Eye size={13} />}
                    </span>
                  )}
                </button>
                <button
                  type="button"
                  data-testid="field-suggestion-fill"
                  title={t('field_suggestions_fill', {
                    object: s.objectName,
                    defaultValue: `Fill into this field with value from "${s.objectName}"`,
                  })}
                  onClick={() => onPick(s.value)}
                  style={{
                    flexShrink: 0,
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 4,
                    margin: 4,
                    padding: '2px 8px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-secondary)',
                    fontSize: 'var(--text-badge)',
                    fontWeight: 600,
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                  }}
                >
                  <ArrowDownLeft size={13} />
                  {t('field_suggestions_fill_short', { defaultValue: '填入' })}
                </button>
              </div>
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

      {/* critical 揭示的主密码验证框（与对象详情共用共享组件） */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={handlePwDialogClose}
        onVerify={handlePwDialogVerify}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        pinAccountId={accountId}
        onPinSuccess={handlePwDialogPinSuccess}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </>
  );
}
