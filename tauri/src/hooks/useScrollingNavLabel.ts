import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { useMediaPreference } from './useMediaPreference';

function subscribeNativePreference(notify: () => void) {
  const observer = new MutationObserver(notify);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-reduce-motion', 'data-high-contrast'],
  });
  return () => observer.disconnect();
}

function nativeStaticLabel() {
  const root = document.documentElement;
  return root.dataset.reduceMotion === 'true' || root.dataset.highContrast === 'true';
}

/** 仅滚动文字层。尺寸/字体/名称变化重新测量，离开、折叠和卸载取消整段动画。 */
export function useScrollingNavLabel(label: string, enabled: boolean, active: boolean) {
  const viewportRef = useRef<HTMLSpanElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);
  const [overflow, setOverflow] = useState(0);
  const [animationUnavailable, setAnimationUnavailable] = useState(false);
  const userReduced = useSettingsStore((s) => s.settings.reduceMotion);
  const systemReduced = useMediaPreference('(prefers-reduced-motion: reduce)');
  const forcedColors = useMediaPreference('(forced-colors: active)');
  const nativeReduced = useSyncExternalStore(
    subscribeNativePreference,
    nativeStaticLabel,
    () => false,
  );
  const staticMode =
    userReduced || systemReduced || forcedColors || nativeReduced || animationUnavailable;

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    const text = textRef.current;
    if (!enabled || !viewport || !text) {
      setOverflow(0);
      return;
    }
    let disposed = false;
    const measure = () => {
      if (disposed) return;
      const width = viewport.getBoundingClientRect().width;
      setOverflow(
        width > 0 ? Math.max(0, Math.ceil(text.getBoundingClientRect().width - width)) : 0,
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    observer.observe(text);
    void document.fonts?.ready.then(measure);
    return () => {
      disposed = true;
      observer.disconnect();
    };
  }, [enabled, label]);

  useEffect(() => {
    const viewport = viewportRef.current;
    const text = textRef.current;
    if (!enabled || !active || !overflow || staticMode || !viewport || !text) return;
    if (typeof text.animate !== 'function') {
      setAnimationUnavailable(true);
      return;
    }
    let stopped = false;
    let animation: Animation | undefined;
    const travelTime = (overflow / 24) * 1000;
    const move = (from: number, to: number, duration: number) => {
      animation?.cancel();
      animation = text.animate(
        [{ transform: `translateX(${from}px)` }, { transform: `translateX(${to}px)` }],
        { duration, easing: 'linear', fill: 'forwards' },
      );
      return animation.finished;
    };
    const loop = async () => {
      while (!stopped) {
        viewport.dataset.fade = 'right';
        viewport.dataset.scrollState = 'waiting';
        await move(0, 0, 400);
        if (stopped) return;
        viewport.dataset.scrollState = 'forward';
        await move(0, -overflow, travelTime);
        if (stopped) return;
        // 末尾停留时去掉右侧遮罩，最后一个字必须完整可读。
        viewport.dataset.fade = 'left';
        viewport.dataset.scrollState = 'tail';
        await move(-overflow, -overflow, 1000);
        if (stopped) return;
        viewport.dataset.fade = 'both';
        viewport.dataset.scrollState = 'reverse';
        await move(-overflow, 0, travelTime);
        if (stopped) return;
        viewport.dataset.fade = 'right';
        viewport.dataset.scrollState = 'rest';
        await move(0, 0, 1000);
      }
    };
    void loop().catch(() => {
      if (!stopped) setAnimationUnavailable(true);
    });
    return () => {
      stopped = true;
      animation?.cancel();
      viewport.dataset.fade = 'right';
      delete viewport.dataset.scrollState;
    };
  }, [enabled, active, overflow, staticMode]);

  return { viewportRef, textRef, isOverflowing: overflow > 0, staticMode };
}
