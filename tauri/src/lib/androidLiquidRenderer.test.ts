import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createAndroidLiquidRenderer,
  type LiquidAppearance,
  type LiquidRenderer,
} from './androidLiquidRenderer';

const appearance: LiquidAppearance = {
  dark: false,
  reduced: false,
  container: '#D9E7F8',
  accent: '#405F82',
  secondary: '#DDE3EA',
  tertiary: '#EDDCF7',
};
const originalResizeObserver = window.ResizeObserver;

function mockWebGl() {
  return {
    VERTEX_SHADER: 0x8b31,
    FRAGMENT_SHADER: 0x8b30,
    COMPILE_STATUS: 0x8b81,
    LINK_STATUS: 0x8b82,
    ARRAY_BUFFER: 0x8892,
    STATIC_DRAW: 0x88e4,
    FLOAT: 0x1406,
    TRIANGLES: 4,
    createShader: vi.fn(() => ({})),
    shaderSource: vi.fn(),
    compileShader: vi.fn(),
    getShaderParameter: vi.fn(() => true),
    createProgram: vi.fn(() => ({})),
    createBuffer: vi.fn(() => ({})),
    attachShader: vi.fn(),
    linkProgram: vi.fn(),
    getProgramParameter: vi.fn(() => true),
    useProgram: vi.fn(),
    bindBuffer: vi.fn(),
    bufferData: vi.fn(),
    getAttribLocation: vi.fn(() => 0),
    enableVertexAttribArray: vi.fn(),
    vertexAttribPointer: vi.fn(),
    getUniformLocation: vi.fn((_program: unknown, name: string) => ({ name })),
    viewport: vi.fn(),
    uniform2f: vi.fn(),
    uniform1f: vi.fn(),
    uniform3f: vi.fn(),
    drawArrays: vi.fn(),
    deleteProgram: vi.fn((_handle: unknown) => {}),
    deleteBuffer: vi.fn((_handle: unknown) => {}),
    deleteShader: vi.fn((_handle: unknown) => {}),
  };
}

describe('Android liquid artwork rendering lifecycle', () => {
  let canvas: HTMLCanvasElement;
  let gl: ReturnType<typeof mockWebGl>;
  let rect: DOMRect;
  let hidden: boolean;
  let frames: Map<number, FrameRequestCallback>;
  let intersect: (visible: boolean) => void;
  let resize: () => void;
  let intersectionDisconnect: ReturnType<typeof vi.fn<() => void>>;
  let resizeDisconnect: ReturnType<typeof vi.fn<() => void>>;
  let renderers: LiquidRenderer[];

  function start(onAvailability = vi.fn()) {
    const renderer = createAndroidLiquidRenderer(canvas, appearance, onAvailability);
    if (renderer) renderers.push(renderer);
    return { renderer, onAvailability };
  }

  function flushFrame() {
    const pending = [...frames.values()];
    frames.clear();
    pending.forEach((callback) => callback(16));
  }

  beforeEach(() => {
    renderers = [];
    gl = mockWebGl();
    canvas = document.createElement('canvas');
    rect = new DOMRect(16, 80, 358, 192);
    hidden = false;
    vi.spyOn(canvas, 'getBoundingClientRect').mockImplementation(() => rect);
    vi.spyOn(canvas, 'getContext').mockReturnValue(gl as unknown as WebGLRenderingContext);
    vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden);
    vi.stubGlobal('innerWidth', 390);
    vi.stubGlobal('innerHeight', 844);
    vi.stubGlobal('devicePixelRatio', 3);
    frames = new Map();
    let nextFrame = 0;
    vi.stubGlobal(
      'requestAnimationFrame',
      vi.fn((callback: FrameRequestCallback) => {
        frames.set(++nextFrame, callback);
        return nextFrame;
      }),
    );
    vi.stubGlobal(
      'cancelAnimationFrame',
      vi.fn((id: number) => frames.delete(id)),
    );
    intersectionDisconnect = vi.fn();
    resizeDisconnect = vi.fn();
    // 不自动触发 observer：首帧不能依赖浏览器稍后才发送的可见性回调。
    vi.stubGlobal(
      'IntersectionObserver',
      class {
        observe = vi.fn();
        disconnect = intersectionDisconnect;
        constructor(callback: IntersectionObserverCallback) {
          intersect = (isIntersecting) =>
            callback(
              [{ isIntersecting, target: canvas } as unknown as IntersectionObserverEntry],
              this as unknown as IntersectionObserver,
            );
        }
      },
    );
    // 共享测试 setup 将此属性定义为可写、不可重新配置，直接替换并在清理时恢复。
    window.ResizeObserver = class {
      observe = vi.fn();
      unobserve = vi.fn();
      disconnect = resizeDisconnect;
      constructor(callback: ResizeObserverCallback) {
        resize = () => callback([], this);
      }
    };
  });

  afterEach(() => {
    renderers.forEach((renderer) => renderer.dispose());
    window.ResizeObserver = originalResizeObserver;
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('可见卡片在 observer 和 RAF 尚未回调时已经绘制，再通知前景显示', () => {
    const { renderer, onAvailability } = start();

    expect(renderer).not.toBeNull();
    expect(gl.drawArrays).toHaveBeenCalledTimes(1);
    expect(onAvailability).toHaveBeenCalledExactlyOnceWith(true);
    expect(gl.drawArrays.mock.invocationCallOrder[0]).toBeLessThan(
      onAvailability.mock.invocationCallOrder[0],
    );
    expect(requestAnimationFrame).not.toHaveBeenCalled();
    expect(frames.size).toBe(0);
    expect([canvas.width, canvas.height]).toEqual([537, 288]);
  });

  it('后台初始化保持 fallback，回到前台后只按需绘制一帧', () => {
    hidden = true;
    const { onAvailability } = start();
    expect(gl.drawArrays).not.toHaveBeenCalled();
    expect(onAvailability).not.toHaveBeenCalledWith(true);

    hidden = false;
    document.dispatchEvent(new Event('visibilitychange'));
    flushFrame();
    expect(gl.drawArrays).toHaveBeenCalledTimes(1);
    expect(onAvailability).toHaveBeenLastCalledWith(true);
    expect(frames.size).toBe(0);
  });

  it('零尺寸卡片保持 fallback，获得实际尺寸且可见后才显示画布', () => {
    rect = new DOMRect(16, 80, 0, 0);
    const { onAvailability } = start();
    expect(gl.drawArrays).not.toHaveBeenCalled();
    expect(onAvailability).not.toHaveBeenCalledWith(true);

    rect = new DOMRect(16, 80, 358, 192);
    intersect(true);
    resize();
    expect(frames.size).toBe(1);
    flushFrame();
    expect(gl.drawArrays).toHaveBeenCalledTimes(1);
    expect(onAvailability).toHaveBeenLastCalledWith(true);
  });

  it('初始位于视口外的卡片不提前消耗绘制，进入视口后恢复', () => {
    rect = new DOMRect(16, 900, 358, 192);
    const { onAvailability } = start();
    expect(gl.drawArrays).not.toHaveBeenCalled();
    expect(onAvailability).not.toHaveBeenCalledWith(true);

    rect = new DOMRect(16, 80, 358, 192);
    intersect(true);
    flushFrame();
    expect(gl.drawArrays).toHaveBeenCalledTimes(1);
  });

  it.each(['context', 'shader', 'program'] as const)(
    '%s 初始化失败时保留 fallback，并释放已创建的资源',
    (failure) => {
      if (failure === 'context') vi.mocked(canvas.getContext).mockReturnValue(null);
      if (failure === 'shader') gl.getShaderParameter.mockReturnValue(false);
      if (failure === 'program') gl.getProgramParameter.mockReturnValue(false);
      const { renderer, onAvailability } = start();

      expect(renderer).toBeNull();
      expect(onAvailability).toHaveBeenCalledExactlyOnceWith(false);
      expect(gl.drawArrays).not.toHaveBeenCalled();
      expect(frames.size).toBe(0);
      expect(gl.deleteShader).toHaveBeenCalledTimes(gl.createShader.mock.calls.length);
      expect(gl.deleteProgram).toHaveBeenCalledTimes(gl.createProgram.mock.calls.length);
      expect(gl.deleteBuffer).toHaveBeenCalledTimes(gl.createBuffer.mock.calls.length);
    },
  );

  it('离屏时取消排队帧并忽略后续重绘，回来时使用最新主题', () => {
    const { renderer } = start();
    renderer!.update({ ...appearance, dark: true });
    expect(frames.size).toBe(1);

    intersect(false);
    expect(frames.size).toBe(0);
    renderer!.update({ ...appearance, dark: true });
    resize();
    canvas.dispatchEvent(new MouseEvent('pointermove', { clientX: 40, clientY: 120 }));
    expect(frames.size).toBe(0);
    expect(gl.drawArrays).toHaveBeenCalledTimes(1);

    intersect(true);
    flushFrame();
    expect(gl.drawArrays).toHaveBeenCalledTimes(2);
    expect(gl.uniform1f).toHaveBeenLastCalledWith({ name: 'dark' }, 1);
    expect(frames.size).toBe(0);
  });

  it('上下文丢失时退回 fallback，恢复后重新建立资源并绘制', () => {
    const { renderer, onAvailability } = start();
    // 模拟原生 WebView 在新上下文拒绝旧句柄，防止恢复成功只看 DOM 状态。
    const stale = new Set([
      ...gl.createProgram.mock.results.map((result) => result.value),
      ...gl.createBuffer.mock.results.map((result) => result.value),
      ...gl.createShader.mock.results.map((result) => result.value),
    ]);
    for (const remove of [gl.deleteProgram, gl.deleteBuffer, gl.deleteShader]) {
      remove.mockImplementation((handle) => {
        if (stale.has(handle)) throw new Error('INVALID_OPERATION: stale context handle');
      });
    }
    renderer!.update(appearance);
    const loss = new Event('webglcontextlost', { cancelable: true });
    canvas.dispatchEvent(loss);

    expect(loss.defaultPrevented).toBe(true);
    expect(onAvailability).toHaveBeenLastCalledWith(false);
    expect(frames.size).toBe(0);
    renderer!.update({ ...appearance, dark: true });
    resize();
    expect(frames.size).toBe(0);

    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(gl.createProgram).toHaveBeenCalledTimes(2);
    flushFrame();
    expect(gl.drawArrays).toHaveBeenCalledTimes(2);
    expect(onAvailability).toHaveBeenLastCalledWith(true);
    expect(gl.uniform1f).toHaveBeenLastCalledWith({ name: 'dark' }, 1);
    expect(frames.size).toBe(0);
  });

  it('卸载取消排队帧、断开观察并释放资源，迟到事件不再绘制', () => {
    const { renderer, onAvailability } = start();
    renderer!.update(appearance);
    renderer!.dispose();

    expect(frames.size).toBe(0);
    expect(intersectionDisconnect).toHaveBeenCalledOnce();
    expect(resizeDisconnect).toHaveBeenCalledOnce();
    expect(gl.deleteProgram).toHaveBeenCalledOnce();
    expect(gl.deleteBuffer).toHaveBeenCalledOnce();
    expect(gl.deleteShader).toHaveBeenCalledTimes(2);

    canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    canvas.dispatchEvent(new MouseEvent('pointerdown', { clientX: 40, clientY: 120 }));
    document.dispatchEvent(new Event('visibilitychange'));
    intersect(true);
    resize();
    renderer!.update(appearance);
    flushFrame();
    expect(gl.createProgram).toHaveBeenCalledOnce();
    expect(gl.drawArrays).toHaveBeenCalledOnce();
    expect(onAvailability).toHaveBeenCalledExactlyOnceWith(true);
  });
});
