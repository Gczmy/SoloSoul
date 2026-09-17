import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Eye, EyeOff } from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ValueContainer } from '@/components/ui/ValueContainer';
import { useRevealState } from '@/hooks/useRevealState';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import type { SensitivityLevel } from '@/types/template';

const levels: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

/** 子字段继承组的保护强度；缺失/未知标签按 internal 处理。 */
export function trashSensitivity(value: unknown, parent?: SensitivityLevel): SensitivityLevel {
  const level = levels.includes(value as SensitivityLevel)
    ? (value as SensitivityLevel)
    : value == null
      ? (parent ?? 'internal')
      : 'internal';
  return parent && levels.indexOf(parent) > levels.indexOf(level) ? parent : level;
}

interface Props {
  identity: string;
  label: string;
  value: string;
  sensitivity?: SensitivityLevel;
}

export function ProtectedTrashValue(props: Props) {
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { identity, value, sensitivity } = props;
  // 字段内容、快照、敏感度或账户变化时重新挂载保护层；旧验证不能揭示新内容。
  const scope = useMemo(
    () => ({ id: crypto.randomUUID(), identity, value, sensitivity, accountId }),
    [identity, value, sensitivity, accountId],
  );
  return <ProtectedValue key={scope.id} {...props} accountId={scope.accountId} />;
}

function ProtectedValue({
  identity,
  label,
  value,
  sensitivity,
  accountId,
}: Props & { accountId?: string }) {
  const { t } = useTranslation(['common', 'sensitivity']);
  const { maskValue, shouldMask, reveal, hide } = useRevealState();
  const [verifying, setVerifying] = useState(false);
  const verificationAttempt = useRef(0);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const level = trashSensitivity(sensitivity);
  const masked = shouldMask(identity, level);
  const display = maskValue(value, identity, level);

  return (
    <>
      <ValueContainer value={display}>
        <span style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{display}</span>
        {level !== 'public' && (
          <button
            type="button"
            className="interactive-icon"
            aria-label={`${label}: ${t(masked ? 'sensitivity:click_to_reveal' : 'sensitivity:hide')}`}
            onClick={() => {
              if (!masked) hide(identity);
              else if (level === 'critical') setVerifying(true);
              else reveal(identity);
            }}
            style={{
              display: 'inline-flex',
              verticalAlign: 'middle',
              marginLeft: 6,
              padding: 3,
              border: 0,
              borderRadius: 4,
              background: 'transparent',
              color: 'var(--text-secondary)',
              cursor: 'pointer',
            }}
          >
            {masked ? <Eye size={14} /> : <EyeOff size={14} />}
          </button>
        )}
      </ValueContainer>
      {verifying && (
        <PasswordVerificationDialog
          open
          onClose={() => {
            verificationAttempt.current += 1;
            setVerifying(false);
          }}
          title={t('common:critical_access_title')}
          description={t('common:critical_access_desc')}
          onVerify={async (password) => {
            const attempt = ++verificationAttempt.current;
            const before = useAuthStore.getState();
            if (!accountId || !before.isAuthenticated || before.currentAccount?.id !== accountId)
              return false;
            const ok = await invoke<boolean>('verify_password', { accountId, password });
            if (
              !ok ||
              !mounted.current ||
              verificationAttempt.current !== attempt ||
              useAuthStore.getState() !== before
            )
              return false;
            reveal(identity);
            void invoke('log_write', {
              request: {
                actionType: 'critical_field_login',
                entityType: 'auth',
                entityId: null,
                entityName: null,
                details: `source=trash fieldName=${label}`,
              },
            }).catch(() => {});
            return true;
          }}
        />
      )}
    </>
  );
}
