import { useCallback, useLayoutEffect, useRef } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { releaseNativeWindowBackdrop } from '@/lib/nativeTitlebarControls';

/** Keep the card mounted until its single native background surface is released. */
export function useNativeBackdropClose(onClose: () => void, open = true) {
  const native = useNativeWindowStore((s) => s.windowsFrame?.fullWindowBackdrop === true);
  const backdropRef = useRef<HTMLDivElement>(null);
  const closing = useRef(false);
  const mounted = useRef(true);
  const openGeneration = useRef(0);
  useLayoutEffect(() => {
    openGeneration.current += 1;
  }, [open]);
  useLayoutEffect(() => {
    mounted.current = true;
    closing.current = false;
    backdropRef.current?.removeAttribute('data-native-backdrop-closing');
    return () => {
      mounted.current = false;
    };
  }, [open, native]);
  const close = useCallback(() => {
    const element = backdropRef.current;
    if (!native || !element) {
      onClose();
      return;
    }
    if (closing.current) return;
    closing.current = true;
    const generation = openGeneration.current;
    void releaseNativeWindowBackdrop(element).finally(() => {
      // A native acknowledgement belongs to this opening, never a reopened card.
      if (mounted.current && generation === openGeneration.current) onClose();
    });
  }, [native, onClose]);
  return { native, backdropRef, close };
}
