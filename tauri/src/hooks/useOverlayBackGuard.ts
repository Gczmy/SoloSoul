import { useCallback, useEffect, useRef } from 'react';

export interface UseOverlayBackGuardOptions {
  /** 内层（如全屏查看器）是否打开：true 时硬件返回先关闭内层，false 时关闭整个浮层。 */
  innerOpen: boolean;
  /** 关闭内层（如全屏查看器回网格）。 */
  onCloseInner: () => void;
  /** 关闭整个浮层（回上一路由/上一页）。 */
  onClose: () => void;
}

/**
 * 模块级：仍由「活跃（已挂载）浮层钩子」认领的标记状态对象集合。
 * 每个钩子压入标记时登记，卸载时释放——释放后该标记即视为「残留」。
 */
const ownedMarkers = new Set<object>();
let sweeperAttached = false;

/**
 * 残留标记跳过监听（模块级单例，首个钩子挂载时注册一次、常驻）。
 *
 * 背景：浮层打开期间若发生 vault 锁定等外部导航（AuthGuard `<Navigate to="/login">`
 * 或 vault-locked 事件 `navigate('/login')`），外部条目会压在本浮层标记之上；卸载
 * 清理因「顶层非本浮层标记」而无法弹出外部条目，标记被埋入历史栈成为残留。此后用户
 * 每次硬件返回都会先弹到一个残留标记（URL 仍是浮层打开时的地址），表现为幻影返回 /
 * 返回卡顿。
 *
 * 方案：popstate 时若落到「无人认领的浮层标记」（solosoulOverlayLayer 且不在
 * ownedMarkers 中），立即再 `history.go(-1)` 跳过——一次返回连续跨越全部残留标记，
 * 直达真实历史条目。活跃钩子自身的标记仍在 ownedMarkers 中，不会被误跳过。
 *
 * 注：依赖 popstate 事件的 state 与 pushState 传入为同一对象引用（浏览器与 jsdom
 * 均保持该引用不变）；此所有权判定若改为结构化克隆将失效。
 */
function attachStaleMarkerSweeper(): void {
  if (sweeperAttached) return;
  sweeperAttached = true;
  window.addEventListener('popstate', (e) => {
    const state = e.state as { solosoulOverlayLayer?: boolean } | null;
    if (state?.solosoulOverlayLayer && !ownedMarkers.has(state)) {
      window.history.go(-1);
    }
  });
}

/**
 * Android/iOS 硬件返回分层守卫（共享 hook）。
 *
 * 问题背景：全屏浮层（照片集、预览器等）以「就地覆盖」打开——URL 不变、也没有
 * 自己的路由历史条目，因此 WebView 的硬件返回会触发 history.back() 跳到浮层打开
 * 前的上一路由（如 /settings），而不是关闭浮层本身。
 *
 * 方案：浮层挂载时压入「浮层层」历史标记（pushState，URL 不变、保留 React Router
 * 的 idx）；内层（如全屏查看器）打开时再压入「内层层」标记。popstate 按层回退：
 * 内层打开 → 关内层（查看器层被弹，回网格），否则 → 关浮层（相册层被弹）。
 * 卸载时仅当顶层仍是本浮层标记才 history.go(-n) 一次性清理残留，避免浮层打开期间
 * 叠加的外部历史条目（如 vault 锁定 navigate('/login')）被误弹——被外部条目埋藏的
 * 标记由模块级 sweeper 在后续返回时自动跳过。
 *
 * 三个相册入口（首页 / 附件管理 GlobalAttachmentManager / 对象详情 AttachmentViewer）
 * 均渲染 PhotoAlbumOverlay，故统一由本 hook 获得「返回关闭相册而非跳路由」行为。
 *
 * @returns handleInnerBack：内层左上角返回按钮的主动返回——内层标记在栈顶时
 *  `history.back()` 触发 popstate 回浮层主体，否则直接关内层（防御性兜底）。
 */
export function useOverlayBackGuard({
  innerOpen,
  onCloseInner,
  onClose,
}: UseOverlayBackGuardOptions): { handleInnerBack: () => void } {
  const innerOpenRef = useRef(innerOpen);
  innerOpenRef.current = innerOpen;
  const onCloseInnerRef = useRef(onCloseInner);
  onCloseInnerRef.current = onCloseInner;
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  const layersRef = useRef(0);
  /** 本钩子压入的所有标记状态对象（供卸载时释放所有权）。 */
  const markerStatesRef = useRef<object[]>([]);

  // 浮层层：挂载压入标记 + popstate 分层处理 + 卸载清理。
  // popstate 时浏览器已弹出顶层标记：内层开着则回浮层主体（内层层被弹），
  // 否则关闭整个浮层（浮层层被弹）。
  useEffect(() => {
    attachStaleMarkerSweeper();
    const prevState = window.history.state as { idx?: number } | null;
    const baseMarker = {
      ...(prevState ?? {}),
      solosoulOverlayLayer: true,
      idx: (prevState?.idx ?? 0) + 1,
    };
    window.history.pushState(baseMarker, '');
    ownedMarkers.add(baseMarker);
    markerStatesRef.current.push(baseMarker);
    layersRef.current += 1;

    const onPopState = () => {
      if (layersRef.current > 0) layersRef.current -= 1;
      if (innerOpenRef.current) {
        onCloseInnerRef.current();
      } else {
        onCloseRef.current();
      }
    };
    window.addEventListener('popstate', onPopState);
    return () => {
      window.removeEventListener('popstate', onPopState);
      // 仅当顶层仍是我们的标记（浮层层/内层层均带 solosoulOverlayLayer）时才清理——
      // 若浮层打开期间叠加了外部历史条目（如 vault 锁定 navigate('/login')），
      // 顶层非标记则跳过，避免误弹外部条目（残留标记交给 sweeper 后续跳过）。
      const top = window.history.state as { solosoulOverlayLayer?: boolean } | null;
      if (top?.solosoulOverlayLayer && layersRef.current > 0) {
        window.history.go(-layersRef.current);
      }
      layersRef.current = 0;
      // 卸载后标记不再属于本钩子：无论已被正常弹出还是被外部条目埋藏，都释放
      // 所有权，使 sweeper 能在用户后续返回时自动跳过残留标记。
      for (const m of markerStatesRef.current) ownedMarkers.delete(m);
      markerStatesRef.current = [];
    };
  }, []);

  // 内层层：内层打开时再压入一层标记（供返回先回浮层主体）。
  // 关闭统一由 popstate / handleInnerBack 负责，本 effect 无需 cleanup。
  useEffect(() => {
    if (!innerOpen) return;
    const prevState = window.history.state as { idx?: number } | null;
    const innerMarker = {
      ...(prevState ?? {}),
      solosoulOverlayLayer: true,
      solosoulOverlayInnerLayer: true,
      idx: (prevState?.idx ?? 0) + 1,
    };
    window.history.pushState(innerMarker, '');
    ownedMarkers.add(innerMarker);
    markerStatesRef.current.push(innerMarker);
    layersRef.current += 1;
  }, [innerOpen]);

  /** 内层左上角返回按钮：内层标记在栈顶时主动弹出（触发 popstate 回浮层主体），
   *  否则直接关内层（防御性兜底）。 */
  const handleInnerBack = useCallback(() => {
    const state = window.history.state as { solosoulOverlayInnerLayer?: boolean } | null;
    if (state?.solosoulOverlayInnerLayer) {
      window.history.back();
    } else {
      onCloseInnerRef.current();
    }
  }, []);

  return { handleInnerBack };
}
