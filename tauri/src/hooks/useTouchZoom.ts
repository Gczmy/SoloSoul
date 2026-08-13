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
  /** 手势作用容器（图片展示区）。容器可能延迟就绪（如组件先以空数据挂载、\n   * 首帧 return null），hook 会在其出现后自动绑定。 */
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
 *   保证捏合不触发页面滚动。手势能否派发到本 hook 取决于容器 touch-action——
 *   调用方须覆写全局 `touch-action: manipulation`（含 pinch-zoom，浏览器会抢手势）
 *   为 pan-y（未放大）让事件完整到达；
 * - 绑定时机：组件可能先以空数据挂载（如 AttachmentPreviewOverlay 的 item 初始为
 *   null，首帧 return null，ref 目标尚不存在），监听不能依赖一次性 effect——每次
 *   渲染后核对 ref 指向，目标元素出现或变化时才重绑；卸载时统一解绑。
 * - 事件处理器仅在首次渲染创建一次（参数经 ref 读取）：bind/unbind 引用身份跨渲染
 *   稳定，removeEventListener 按回调引用匹配，身份不稳定会导致解绑静默失效。
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

  // 最新值引用：监听器在元素出现时绑定，事件回调内读取 ref 避免过期闭包
  const latest = useRef({ scale, setScale, fitScale, fitToView });
  latest.current = { scale, setScale, fitScale, fitToView };

  // 可变参数（min/max/doubleTapFactor）：稳定处理器经此读取，props 变化也能感知
  const paramsRef = useRef({ minScale, maxScale, doubleTapFactor });
  paramsRef.current = { minScale, maxScale, doubleTapFactor };

  // 捏合过程中同步写入的当前比例：touchend 时 state 可能尚未刷新（React 批量更新），
  // 回弹判定必须读此值而不是 latest.current.scale
  const currentScaleRef = useRef(scale);
  currentScaleRef.current = scale;

  const pinchRef = useRef<{ startDist: number; startScale: number } | null>(null);
  const lastTapTimeRef = useRef(0);
  const tapStartRef = useRef<{ x: number; y: number; t: number } | null>(null);

  // 当前已绑定监听的元素：绑定与解绑以它为准，避免重复绑定
  const boundElRef = useRef<HTMLElement | null>(null);

  // 事件处理器只创建一次：bind/unbind 身份稳定，卸载/换元素时 removeEventListener
  // 才能按回调引用匹配到实际挂载的监听（否则静默解绑失败、监听泄漏）。
  const stableRef = useRef<{
    onTouchStart: (e: TouchEvent) => void;
    onTouchMove: (e: TouchEvent) => void;
    onTouchEnd: (e: TouchEvent) => void;
    bind: (el: HTMLElement) => void;
    unbind: (el: HTMLElement) => void;
  } | null>(null);
  if (!stableRef.current) {
    const clamp = (v: number) =>
      Math.min(paramsRef.current.maxScale, Math.max(paramsRef.current.minScale, v));

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
              latest.current.setScale(
                clamp(latest.current.fitScale * paramsRef.current.doubleTapFactor),
              );
            } else {
              latest.current.fitToView();
            }
          } else {
            lastTapTimeRef.current = now;
          }
        }
      }
    };

    const bind = (el: HTMLElement) => {
      el.addEventListener('touchstart', onTouchStart, { passive: true });
      el.addEventListener('touchmove', onTouchMove, { passive: false });
      el.addEventListener('touchend', onTouchEnd);
      el.addEventListener('touchcancel', onTouchEnd);
    };

    const unbind = (el: HTMLElement) => {
      el.removeEventListener('touchstart', onTouchStart);
      el.removeEventListener('touchmove', onTouchMove);
      el.removeEventListener('touchend', onTouchEnd);
      el.removeEventListener('touchcancel', onTouchEnd);
    };

    stableRef.current = { onTouchStart, onTouchMove, onTouchEnd, bind, unbind };
  }

  // 无依赖数组：每次渲染后核对 ref 指向。目标元素从无到有（延迟就绪）或切换时
  // 才重绑；元素不变则什么都不做，避免重复绑定/解绑抖动。
  // （stableRef 在首次渲染必然初始化，此处经 ! 断言取用。）
  useEffect(() => {
    const { bind, unbind } = stableRef.current!;
    const el = elementRef.current;
    if (!el || el === boundElRef.current) return;
    if (boundElRef.current) unbind(boundElRef.current);
    boundElRef.current = el;
    bind(el);
  });

  // 组件卸载：解绑当前元素并清标记（stableRef 身份稳定，经同一引用解绑）
  useEffect(() => {
    return () => {
      const { unbind } = stableRef.current!;
      if (boundElRef.current) {
        unbind(boundElRef.current);
        boundElRef.current = null;
      }
    };
  }, []);

  return { pinchActive };
}
