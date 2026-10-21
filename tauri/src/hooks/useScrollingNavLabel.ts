import { useLayoutEffect, useRef, useState, useSyncExternalStore } from 'react';
import { useSettingsStore } from '@/stores/settingsStore';
import { useMediaPreference } from './useMediaPreference';
import { followsSystemMotion } from '@/lib/motionPolicy';

function subscribeNativePreference(notify: () => void) {
  const observer = new MutationObserver(notify);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-reduce-motion', 'data-high-contrast', 'data-motion-policy'],
  });
  return () => observer.disconnect();
}

function nativeStaticLabel() {
  const root = document.documentElement;
  return (
    (followsSystemMotion() && root.dataset.reduceMotion === 'true') ||
    root.dataset.highContrast === 'true'
  );
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
    userReduced ||
    (followsSystemMotion() && systemReduced) ||
    forcedColors ||
    nativeReduced ||
    animationUnavailable;

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

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    const text = textRef.current;
    if (!enabled || !active || !overflow || staticMode || !viewport || !text) return;
    if (typeof text.animate !== 'function') {
      setAnimationUnavailable(true);
      return;
    }
    // 一次创建完整往返时间轴，停留/折返不再取消和重建文字的合成动画。
    const delay = 400;
    const rampTime = Math.min(180, ((overflow / 24) * 1000) / 2);
    const rampDistance = (24 * rampTime) / 2000;
    const travelTime = (overflow / 24) * 1000 + rampTime;
    const tailEnd = travelTime + 1000;
    const reverseEnd = tailEnd + travelTime;
    const duration = reverseEnd + 1000;
    const accelerate = 'cubic-bezier(0.333333, 0, 0.666667, 0.333333)';
    const decelerate = 'cubic-bezier(0.333333, 0.666667, 0.666667, 1)';
    const frame = (time: number, x: number, easing = 'linear'): Keyframe => ({
      offset: time / duration,
      transform: `translate3d(${x}px, 0, 0)`,
      easing,
    });
    let animation: Animation;
    try {
      animation = text.animate(
        [
          frame(0, 0, accelerate),
          frame(rampTime, -rampDistance),
          frame(travelTime - rampTime, -overflow + rampDistance, decelerate),
          frame(travelTime, -overflow),
          frame(tailEnd, -overflow, accelerate),
          frame(tailEnd + rampTime, -overflow + rampDistance),
          frame(reverseEnd - rampTime, -rampDistance, decelerate),
          frame(reverseEnd, 0),
          frame(duration, 0),
        ],
        { delay, duration, iterations: Infinity, fill: 'both' },
      );
    } catch {
      setAnimationUnavailable(true);
      return;
    }
    let tick = 0;
    const updatePhase = () => {
      // 只观察时间来切换边缘遮罩，不用 JS 逐帧设置位移或测量布局。
      const time = (typeof animation.currentTime === 'number' ? animation.currentTime : 0) - delay;
      const position = Math.max(0, time) % duration;
      const phase =
        time < 0
          ? 'waiting'
          : position < travelTime
            ? 'forward'
            : position < tailEnd
              ? 'tail'
              : position < reverseEnd
                ? 'reverse'
                : 'rest';
      if (viewport.dataset.scrollState !== phase) {
        viewport.dataset.scrollState = phase;
        // 末尾停留时去掉右侧遮罩，最后一个字必须完整可读。
        viewport.dataset.fade = phase === 'tail' ? 'left' : phase === 'reverse' ? 'both' : 'right';
      }
      tick = requestAnimationFrame(updatePhase);
    };
    updatePhase();
    return () => {
      cancelAnimationFrame(tick);
      animation.cancel();
      viewport.dataset.fade = 'right';
      delete viewport.dataset.scrollState;
    };
  }, [enabled, active, label, overflow, staticMode]);

  return { viewportRef, textRef, isOverflowing: overflow > 0, staticMode };
}
