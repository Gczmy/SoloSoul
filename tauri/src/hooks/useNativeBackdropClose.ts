import { useCallback, useLayoutEffect, useRef } from 'react';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { releaseNativeWindowBackdrop } from '@/lib/nativeTitlebarControls';

/** Keep the card mounted until its single native background surface is released. */
export function useNativeBackdropClose(onClose: () => void, open = true) {
  const native = useNativeWindowStore((s) => s.windowsFrame?.fullWindowBackdrop === true);
  const backdropRef = useRef<HTMLDivElement>(null);
  const closing = useRef(false);
  const mounted = useRef(true);
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
    void releaseNativeWindowBackdrop(element).finally(() => {
      if (mounted.current) onClose();
    });
  }, [native, onClose]);
  return { native, backdropRef, close };
}
