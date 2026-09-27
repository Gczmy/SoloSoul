import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Eye, EyeOff } from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ProtectedFieldValue } from '@/components/ui/ProtectedFieldValue';
import { ValueContainer } from '@/components/ui/ValueContainer';
import { fieldPresentationIdentity, fieldPresentationPolicy } from '@/lib/fieldPresentationPolicy';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { useAuthStore } from '@/stores/authStore';
import type { SensitivityLevel } from '@/types/template';

/** 兼容回收站调用方；等级与父级规则只由共享策略决定。 */
export function trashSensitivity(value: unknown, parent?: SensitivityLevel): SensitivityLevel {
  return fieldPresentationPolicy({ fieldId: '', definition: { sensitivityLevel: value }, parent })
    .sensitivity;
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
  // 业务授权对话框与共享保护层使用相同身份边界，旧 resolver 不能授权新内容。
  const scope = fieldPresentationIdentity(accountId, identity, 'value', [
    value,
    trashSensitivity(sensitivity),
  ]);
  return <ProtectedValue key={scope} {...props} accountId={accountId} />;
}

function ProtectedValue({
  identity,
  label,
  value,
  sensitivity,
  accountId,
}: Props & { accountId?: string }) {
  const { t } = useTranslation(['common', 'sensitivity']);
  const policy = fieldPresentationPolicy({
    fieldId: identity,
    definition: { sensitivityLevel: sensitivity },
  });
  const [requests] = useState(createSessionRequests);
  const [verifying, setVerifying] = useState(false);
  const resolveRef = useRef<((ok: boolean) => void) | null>(null);
  const close = useCallback(() => {
    requests.invalidate();
    resolveRef.current?.(false);
    resolveRef.current = null;
    setVerifying(false);
  }, [requests]);
  useLayoutEffect(() => {
    const unsubscribe = onRequestSessionChange(close);
    return () => {
      unsubscribe();
      close();
    };
  }, [close]);
  const canAuthorize = () => {
    const auth = useAuthStore.getState();
    return !!accountId && auth.isAuthenticated && auth.currentAccount?.id === accountId;
  };
  const authorize = async () => {
    if (!canAuthorize()) return false;
    if (!policy.requiresVerification) return true;
    close();
    setVerifying(true);
    return new Promise<boolean>((resolve) => {
      resolveRef.current = resolve;
    });
  };

  return (
    <>
      <ProtectedFieldValue
        accountId={accountId}
        objectId={identity}
        fieldId="value"
        value={value}
        policy={policy}
        authorize={authorize}
      >
        {(control) => (
          <ValueContainer
            value={control.displayValue}
            action={
              policy.concealed && (
                <button
                  type="button"
                  className="interactive-icon"
                  aria-label={`${label}: ${t(control.revealed ? 'sensitivity:hide' : 'sensitivity:click_to_reveal')}`}
                  onClick={() => {
                    if (control.revealed) {
                      close();
                      control.hide();
                    } else void control.reveal();
                  }}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                    padding: 3,
                    border: 0,
                    borderRadius: 4,
                    background: 'transparent',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                  }}
                >
                  {control.revealed ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              )
            }
          >
            <span style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>
              {control.displayValue}
            </span>
          </ValueContainer>
        )}
      </ProtectedFieldValue>
      {verifying && (
        <PasswordVerificationDialog
          open
          onClose={close}
          title={t('common:critical_access_title')}
          description={t('common:critical_access_desc')}
          onVerify={async (password) => {
            const resolve = resolveRef.current;
            const request = requests.begin('verify', accountId);
            if (!resolve || !canAuthorize() || !request.isCurrent()) return false;
            let ok: boolean;
            try {
              ok = await request.invoke<boolean>('verify_password', { accountId, password });
            } catch (error) {
              if (!request.isCurrent() || !canAuthorize()) return false;
              throw error;
            }
            if (!ok || !request.isCurrent() || !canAuthorize() || resolveRef.current !== resolve)
              return false;
            try {
              await request.invoke('log_write', {
                request: {
                  actionType: 'critical_field_login',
                  entityType: 'auth',
                  entityId: null,
                  entityName: null,
                  details: `source=trash fieldName=${label}`,
                },
              });
            } catch {
              // 审计保持 best effort；过期的授权仍须拒绝。
            }
            if (!request.isCurrent() || !canAuthorize() || resolveRef.current !== resolve)
              return false;
            resolveRef.current = null;
            resolve(true);
            return true;
          }}
        />
      )}
    </>
  );
}
