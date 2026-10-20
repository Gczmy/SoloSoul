import { useLayoutEffect, useState, useSyncExternalStore, type RefObject } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { useMediaPreference } from '@/hooks/useMediaPreference';

function subscribeNativeMotion(notify: () => void) {
  const observer = new MutationObserver(notify);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-reduce-motion'],
  });
  return () => observer.disconnect();
}

/** Enable transitions in the same React commit as the open state, never during width dragging. */
export function usePagePaneMotion(
  body: RefObject<HTMLDivElement | null>,
  enabled: boolean,
  open: boolean,
) {
  const userReduced = useSettingsStore((state) => state.settings.reduceMotion);
  const systemReduced = useMediaPreference('(prefers-reduced-motion: reduce)');
  const nativeReduced = useSyncExternalStore(
    subscribeNativeMotion,
    () => document.documentElement.dataset.reduceMotion === 'true',
    () => false,
  );
  const reduced = userReduced || systemReduced || nativeReduced;
  const [previous, setPrevious] = useState({ enabled, open, reduced });
  const [animating, setAnimating] = useState(false);
  if (previous.enabled !== enabled || previous.open !== open || previous.reduced !== reduced) {
    setPrevious({ enabled, open, reduced });
    setAnimating(enabled && previous.enabled && previous.open !== open && !reduced);
  }

  useLayoutEffect(() => {
    if (!animating || !body.current) return;
    let cancelled = false;
    const frame = requestAnimationFrame(() => {
      const animations =
        body.current
          ?.getAnimations?.()
          .filter(
            (animation) =>
              (animation as CSSTransition).transitionProperty === 'grid-template-columns',
          ) ?? [];
      void Promise.allSettled(animations.map((animation) => animation.finished)).then(() => {
        if (!cancelled) setAnimating(false);
      });
    });
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [body, open, animating]);

  return animating && !reduced;
}
