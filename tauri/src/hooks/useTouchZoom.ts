import { useEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';

/** 触点坐标（Touch 的 clientX/clientY 子集，便于测试注入普通对象）。 */
interface Point {
  clientX: number;
  clientY: number;
}

/** 两点间距离。 */
function distance(a: Point, b: Point): number {
  return Math.hypot(b.clientX - a.clientX, b.clientY - a.clientY);
}

/** 双击判定：两次点按间隔（ms）。 */
const DOUBLE_TAP_WINDOW = 300;
/** 点按判定：按下到抬起最大时长（ms）。 */
const TAP_MAX_DURATION = 250;
/** 点按判定：允许的最大位移（px），超过视为拖动。 */
const TAP_MAX_MOVE = 10;

export interface UseTouchZoomOptions {
  /** 手势作用容器（图片展示区）。容器应在组件生命周期内稳定；touch 事件经其冒泡到达。 */
  elementRef: RefObject<HTMLElement | null>;
  /** 当前缩放比例（相对图片原始尺寸）。 */
  scale: number;
  /** 设置缩放比例（绝对赋值）。 */
  setScale: (next: number) => void;
  /** 适应视口比例：双击以此为基准放大、捏合回落低于它时回弹保持整图。 */
  fitScale: number;
  /** 回退到适应视口。 */
  fitToView: () => void;
  /** 缩放下限 / 上限；默认 0.1 / 5。 */
  minScale?: number;
  maxScale?: number;
  /** 双击放大倍率（相对 fit）；默认 2。 */
  doubleTapFactor?: number;
}

export interface TouchZoomState {
  /** 双指捏合进行中：组件应暂停与捏合冲突的其它手势（如 swipe 翻页）。 */
  pinchActive: boolean;
}

/**
 * 图片双指捏合缩放 + 双击缩放（T010，安卓端手势）。
 *
 * - 捏合：新比例 = 起始比例 × (当前两指距离 / 起始两指距离)，clamp 至 [minScale, maxScale]；
 *   任一指抬起即结束，若结束时比例低于 fit（缩小过头）回弹到 fit 保持整图可见；
 * - 双击：DOUBLE_TAP_WINDOW 内两次快速点按（无位移、时长 ≤ TAP_MAX_DURATION）→
 *   在 fit 与 fit×doubleTapFactor 之间切换；
 * - 监听以原生 touch 事件绑定：React 合成 touch 事件在根上是被动监听无法
 *   preventDefault，这里以 `{ passive: false }` 绑定 touchmove 拦截浏览器滚动/手势，
 *   保证捏合不触发页面滚动（全局 `touch-action: manipulation` + `user-scalable=no`
 *   下浏览器双指缩放已被禁用，事件可完整到达本 hook）。
 */
export function useTouchZoom({
  elementRef,
  scale,
  setScale,
  fitScale,
  fitToView,
  minScale = 0.1,
  maxScale = 5,
  doubleTapFactor = 2,
}: UseTouchZoomOptions): TouchZoomState {
  const [pinchActive, setPinchActive] = useState(false);

  // 最新值引用：监听器只在元素挂载时绑定一次，事件回调内读取 ref 避免过期闭包
  const latest = useRef({ scale, setScale, fitScale, fitToView });
  latest.current = { scale, setScale, fitScale, fitToView };

  // 捏合过程中同步写入的当前比例：touchend 时 state 可能尚未刷新（React 批量更新），
  // 回弹判定必须读此值而不是 latest.current.scale
  const currentScaleRef = useRef(scale);
  currentScaleRef.current = scale;

  const pinchRef = useRef<{ startDist: number; startScale: number } | null>(null);
  const lastTapTimeRef = useRef(0);
  const tapStartRef = useRef<{ x: number; y: number; t: number } | null>(null);

  useEffect(() => {
    const el = elementRef.current;
    if (!el) return;

    const clamp = (v: number) => Math.min(maxScale, Math.max(minScale, v));

    const onTouchStart = (e: TouchEvent) => {
      if (e.touches.length === 2) {
        // 双指落下：新捏合。同时作废尚未完成单击判定的 tap 候选，
        // 避免「点按后紧接着捏合」在最后一指抬起时误触发双击缩放。
        tapStartRef.current = null;
        pinchRef.current = {
          startDist: distance(e.touches[0], e.touches[1]),
          startScale: currentScaleRef.current,
        };
        setPinchActive(true);
        return;
      }
      if (e.touches.length === 1) {
        tapStartRef.current = {
          x: e.touches[0].clientX,
          y: e.touches[0].clientY,
          t: Date.now(),
        };
      }
    };

    const onTouchMove = (e: TouchEvent) => {
      const pinch = pinchRef.current;
      if (pinch && e.touches.length === 2) {
        const dist = distance(e.touches[0], e.touches[1]);
        if (dist > 0) {
          const next = clamp(pinch.startScale * (dist / pinch.startDist));
          currentScaleRef.current = next;
          latest.current.setScale(next);
        }
        // 阻止浏览器在捏合期间滚动页面
        e.preventDefault();
        return;
      }
      // 单指位移超过阈值 → 判定为拖动而非点按，取消双击候选
      if (e.touches.length === 1 && tapStartRef.current) {
        const t = e.touches[0];
        if (
          Math.hypot(t.clientX - tapStartRef.current.x, t.clientY - tapStartRef.current.y) >
          TAP_MAX_MOVE
        ) {
          tapStartRef.current = null;
        }
      }
    };

    const onTouchEnd = (e: TouchEvent) => {
      // 手势被浏览器接管（系统手势/边缘滑动等）：touchcancel 的 touches 可能仍报 2 指，
      // 普通 touchend 分支的 touches.length < 2 条件不会命中 → 无条件清理，
      // 否则 pinchActive 永久卡 true，滑动翻页与拖动被禁用直至重挂载。
      if (e.type === 'touchcancel') {
        pinchRef.current = null;
        tapStartRef.current = null;
        setPinchActive(false);
        return;
      }
      const wasPinching = pinchRef.current !== null;
      if (wasPinching && e.touches.length < 2) {
        // 任一指抬起 → 捏合结束
        pinchRef.current = null;
        setPinchActive(false);
        // 缩小过头（低于 fit）→ 回弹 fit，保持整图可见（同时恢复 swipe/整图语义）
        if (currentScaleRef.current <= latest.current.fitScale) {
          latest.current.fitToView();
        }
        return;
      }
      // 双击判定：单指快速两次点按（touches 已清零、无位移、时长在窗口内）
      if (!wasPinching && e.touches.length === 0 && tapStartRef.current) {
        const start = tapStartRef.current;
        tapStartRef.current = null;
        const now = Date.now();
        if (now - start.t <= TAP_MAX_DURATION) {
          if (lastTapTimeRef.current !== 0 && now - lastTapTimeRef.current <= DOUBLE_TAP_WINDOW) {
            lastTapTimeRef.current = 0;
            const cur = currentScaleRef.current;
            if (cur <= latest.current.fitScale + 0.001) {
              latest.current.setScale(clamp(latest.current.fitScale * doubleTapFactor));
            } else {
              latest.current.fitToView();
            }
          } else {
            lastTapTimeRef.current = now;
          }
        }
      }
    };

    el.addEventListener('touchstart', onTouchStart, { passive: true });
    el.addEventListener('touchmove', onTouchMove, { passive: false });
    el.addEventListener('touchend', onTouchEnd);
    el.addEventListener('touchcancel', onTouchEnd);
    return () => {
      el.removeEventListener('touchstart', onTouchStart);
      el.removeEventListener('touchmove', onTouchMove);
      el.removeEventListener('touchend', onTouchEnd);
      el.removeEventListener('touchcancel', onTouchEnd);
    };
  }, [elementRef, minScale, maxScale, doubleTapFactor]);

  return { pinchActive };
}
