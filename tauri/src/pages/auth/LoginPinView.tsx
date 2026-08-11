import { PinEntryCard } from '@/components/forms/PinEntryCard';
import type { PinInputHandle } from '@/components/forms/PinInput';
import type { RefObject } from 'react';

interface LoginPinViewProps {
  pinUnlocking: boolean;
  pinError: string | null;
  pinInputKey: number;
  pinInputRef: RefObject<PinInputHandle | null>;
  onPinComplete: (pin: string) => void;
}

/** PIN 解锁面板（登录页，P040: 复用共享 PinEntryCard）。 */
export function LoginPinView({
  pinUnlocking,
  pinError,
  pinInputKey,
  pinInputRef,
  onPinComplete,
}: LoginPinViewProps) {
  return (
    <PinEntryCard
      pinUnlocking={pinUnlocking}
      pinError={pinError}
      pinInputKey={pinInputKey}
      pinInputRef={pinInputRef}
      onPinComplete={onPinComplete}
      onCardClick={() => pinInputRef.current?.focus()}
      marginBottom={16}
    />
  );
}
