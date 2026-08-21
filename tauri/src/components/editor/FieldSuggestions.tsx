import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDownLeft, ChevronDown, ChevronUp, Eye, EyeOff, Sparkles } from 'lucide-react';
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
  /** 折叠态最多展示条数（超出时提供展开/收起按钮）。 */
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
 * - critical：掩码，点击弹出主密码验证框（支持密码/PIN/生物识别），验证成功后才揭示；
 *   解锁后 1 分钟宽限期内再次查看/填入同一条目无需重复验证。
 *
 * 揭示中的条目右侧显示自动隐藏倒计时（剩余秒数，每秒更新），到期自动重掩。
 *
 * 每行右侧「填入」按钮将真实值回填到当前字段：公开/内部/敏感或已揭示时直接填入；
 * critical 未揭示时先弹主密码验证框（与查看同款），验证成功后直接回填、无需再次点击；
 * 解锁后 1 分钟宽限期内再次填入同一条目无需重复验证。
 * 提示文案与揭示按钮保持一致：critical 未揭示时点「填入」同样先验证。
 *
 * 超过 limit（默认 3）条时折叠展示，底部提供展开/收起按钮查看其余条目。
 */
export function FieldSuggestions({
  fieldName,
  suggestions,
  onPick,
  limit = 3,
}: FieldSuggestionsProps) {
  const { t } = useTranslation(['editor', 'common']);
  const [expanded, setExpanded] = useState(false);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    isRevealed,
    revealRemainingMs,
    handleItemClick,
    handleFillClick,
    showPwDialog,
    handlePwDialogClose,
    handlePwDialogVerify,
    handlePwDialogPinSuccess,
    passwordHint,
    bioAvailable,
    handleBiometricUnlock,
  } = useSuggestionReveal(accountId);

  // 揭示中的条目展示自动隐藏倒计时（每秒跳动，驱动重渲染；具体秒数渲染时实时计算）
  const [, setTick] = useState(0);
  const anyRevealed = suggestions.some((s) => {
    const level = (s.sensitivityLevel as SensitivityLevel) || 'internal';
    return (level === 'sensitive' || level === 'critical') && isRevealed(suggestionItemId(s));
  });
  useEffect(() => {
    if (!anyRevealed) return;
    const timer = window.setInterval(() => setTick((t) => t + 1), 1000);
    return () => window.clearInterval(timer);
  }, [anyRevealed]);

  if (!suggestions || suggestions.length === 0) return null;

  const hasMore = suggestions.length > limit;
  const shown = expanded ? suggestions : suggestions.slice(0, limit);
  const hiddenCount = expanded ? 0 : suggestions.length - limit;

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
            const seconds = Math.max(0, Math.ceil(revealRemainingMs(itemId) / 1000));
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
                    <>
                      {revealed && (
                        <span
                          title={t('field_suggestions_reveal_countdown_title', {
                            seconds,
                            defaultValue: `Auto-hides in ${seconds}s`,
                          })}
                          data-testid="field-suggestion-countdown"
                          style={{
                            flexShrink: 0,
                            color: 'var(--text-tertiary)',
                            fontSize: 'var(--text-badge)',
                            fontVariantNumeric: 'tabular-nums',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {t('field_suggestions_reveal_countdown', {
                            seconds,
                            defaultValue: `${seconds}s`,
                          })}
                        </span>
                      )}
                      <span
                        style={{ flexShrink: 0, color: 'var(--text-tertiary)', display: 'flex' }}
                      >
                        {revealed ? <EyeOff size={13} /> : <Eye size={13} />}
                      </span>
                    </>
                  )}
                </button>
                <button
                  type="button"
                  data-testid="field-suggestion-fill"
                  title={t('field_suggestions_fill', {
                    object: s.objectName,
                    defaultValue: `Fill into this field with value from "${s.objectName}"`,
                  })}
                  onClick={() => handleFillClick(s, onPick)}
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
          {hasMore && (
            <button
              type="button"
              data-testid="field-suggestions-toggle"
              onClick={() => setExpanded((v) => !v)}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 4,
                padding: '4px 8px',
                borderRadius: 6,
                border: 'none',
                background: 'transparent',
                color: 'var(--text-secondary)',
                fontSize: 'var(--text-badge)',
                fontWeight: 600,
                cursor: 'pointer',
                alignSelf: 'flex-start',
              }}
            >
              {expanded ? (
                <>
                  <ChevronUp size={13} />
                  {t('field_suggestions_collapse', { defaultValue: 'Collapse' })}
                </>
              ) : (
                <>
                  <ChevronDown size={13} />
                  {t('field_suggestions_expand', {
                    n: hiddenCount,
                    defaultValue: `Expand (${hiddenCount} more)`,
                  })}
                </>
              )}
            </button>
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
