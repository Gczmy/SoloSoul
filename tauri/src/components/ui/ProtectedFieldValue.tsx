import { useLayoutEffect, useState, type ReactNode } from 'react';
import { useRevealState } from '@/hooks/useRevealState';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import {
  fieldPresentationIdentity,
  protectedDisplayValue,
  type FieldPresentationPolicy,
} from '@/lib/fieldPresentationPolicy';

export interface ProtectedFieldControl {
  displayValue: string;
  revealed: boolean;
  remainingMs: number;
  reveal: () => Promise<boolean>;
  copy: (value?: string, key?: string) => Promise<void>;
}
interface Props {
  accountId?: string;
  objectId: string;
  fieldId: string;
  value: string;
  contentVersion?: unknown;
  policy: FieldPresentationPolicy;
  authorize: () => Promise<boolean>;
  onCopy?: (value: string, key: string) => void | Promise<void>;
  children?: (control: ProtectedFieldControl) => ReactNode;
}

/** 共享保护值 UI：无业务 Store 依赖，验证和复制由调用方提供。 */
export function ProtectedFieldValue(props: Props) {
  const identity = fieldPresentationIdentity(props.accountId, props.objectId, props.fieldId, [
    props.value,
    props.policy.sensitivity,
    props.contentVersion,
  ]);
  return <ProtectedFieldSession key={identity} {...props} />;
}

function ProtectedFieldSession({
  accountId,
  fieldId,
  value,
  policy,
  authorize,
  onCopy,
  children,
}: Props) {
  const state = useRevealState();
  const [requests] = useState(createSessionRequests);
  const { hide } = state;
  useLayoutEffect(() => {
    const unsubscribe = onRequestSessionChange(() => {
      requests.invalidate();
      hide('value');
    });
    return () => {
      unsubscribe();
      requests.invalidate();
    };
  }, [requests, hide]);
  const revealed = state.isRevealed('value');
  const [, setTick] = useState(0);
  useLayoutEffect(() => {
    if (!revealed) return;
    const timer = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(timer);
  }, [revealed]);
  const access = async () => {
    const request = requests.begin('access', accountId);
    if (policy.concealed && !state.isRevealed('value')) {
      try {
        if (!(await authorize()) || !request.isCurrent()) return null;
      } catch {
        return null;
      }
      state.reveal('value');
    }
    return request.isCurrent() ? request : null;
  };
  const control: ProtectedFieldControl = {
    displayValue: protectedDisplayValue(value, policy.sensitivity, revealed),
    revealed,
    remainingMs: state.revealRemainingMs('value'),
    reveal: async () => !!(await access()),
    copy: async (copyValue = value, key = fieldId) => {
      const request = await access();
      if (request?.isCurrent()) await onCopy?.(copyValue, key);
    },
  };
  return children ? children(control) : <span>{control.displayValue}</span>;
}
