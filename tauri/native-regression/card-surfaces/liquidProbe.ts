// FE2 原生绘制补证：生产渲染器与 CSS，合成文字；不加载账户或应用初始化。
import { applyAndroidMaterial, getAndroidMaterialSnapshot } from '../../src/lib/androidMaterial';
import { getSchemeById } from '../../src/lib/themeSchemes';
import {
  createAndroidLiquidRenderer,
  type LiquidRenderer,
} from '../../src/lib/androidLiquidRenderer';

let renderer: LiquidRenderer | null = null;
let surface: HTMLElement | null = null;
let canvas: HTMLCanvasElement | null = null;
let lossExtension: WEBGL_lose_context | null = null;
let draws = 0;
let ready = false;
let paintFrames = 0;
let generation = 0;
let gpu: { uniforms: Record<string, number[]>; pixel: number[]; error: number } | null = null;

export function startLiquidProbe(theme: 'light' | 'dark', accent: string) {
  if (!/^#[\da-f]{6}$/i.test(accent)) throw new Error('Probe accent must be numeric hex');
  renderer?.dispose();
  surface?.remove();
  draws = 0;
  ready = false;
  paintFrames = 0;
  const current = ++generation;
  gpu = null;
  lossExtension = null;
  const root = document.documentElement;
  root.dataset.platform = 'android';
  delete root.dataset.desktopPlatform;
  root.dataset.theme = theme;
  root.dataset.androidGlass = 'enhanced';
  const scheme = getSchemeById(theme === 'light' ? 'warm-stone' : 'warm-stone-dark')!;
  Object.entries(scheme.variables).forEach(([key, value]) => root.style.setProperty(key, value));
  root.style.setProperty('--accent-primary', accent);
  applyAndroidMaterial();
  document.getElementById('root')!.style.display = 'none';
  surface = document.createElement('section');
  surface.className = 'android-overview';
  surface.dataset.liquid = 'true';
  surface.style.margin = '32px 16px';
  // 固定的公开测试文本。生产样式决定位置与表面；不伪装成完整 AndroidHome。
  surface.innerHTML =
    '<div class="android-liquid-artwork"><div class="android-liquid-fallback"></div><canvas></canvas></div><div class="android-overview-copy"><p>Native renderer probe</p><div class="android-overview-stat"><strong>42</strong><span>objects</span></div><p>Synthetic public text</p></div>';
  document.body.append(surface);
  window.scrollTo(0, 0);
  canvas = surface.querySelector('canvas')!;
  const { dark, colors } = getAndroidMaterialSnapshot();
  renderer = createAndroidLiquidRenderer(
    canvas,
    {
      dark,
      reduced: true,
      container: colors['--android-liquid-base'],
      accent: colors['--accent-primary'],
      secondary: colors['--md-secondary-container'],
      tertiary: colors['--md-tertiary-container'],
    },
    (available) => {
      ready = available;
      surface!.querySelector<HTMLElement>('.android-liquid-artwork')!.dataset.liquidReady =
        String(available);
      if (!available) return;
      draws += 1;
      const gl = canvas!.getContext('webgl')!;
      const program = gl.getParameter(gl.CURRENT_PROGRAM) as WebGLProgram;
      const uniforms = Object.fromEntries(
        ['container', 'accent', 'secondary', 'tertiary'].map((key) => {
          const location = gl.getUniformLocation(program, key);
          if (!location) throw new Error(`Missing production uniform: ${key}`);
          return [key, Array.from(gl.getUniform(program, location) as Float32Array)];
        }),
      );
      // 在生产 draw 回调中读取，避免 preserveDrawingBuffer=false 呈现后被清空。
      const pixel = new Uint8Array(4);
      gl.readPixels(
        Math.floor(canvas!.width / 2),
        Math.floor(canvas!.height / 2),
        1,
        1,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        pixel,
      );
      gpu = { uniforms, pixel: Array.from(pixel), error: gl.getError() };
    },
  );
  // 原生截图必须等待本轮 DOM 的真实帧；GPU draw 回调不等于合成已呈现。
  const painted = () => {
    if (current !== generation) return;
    paintFrames += 1;
    if (paintFrames < 8) requestAnimationFrame(painted);
  };
  requestAnimationFrame(painted);
}

export function readLiquidProbe() {
  if (!surface || !canvas) throw new Error('Probe not started');
  const copy = surface.querySelector<HTMLElement>('.android-overview-copy')!;
  const art = surface.querySelector<HTMLElement>('.android-liquid-artwork')!;
  const text = copy.getBoundingClientRect(),
    decoration = art.getBoundingClientRect();
  const rect = surface.getBoundingClientRect();
  return {
    ready,
    paintFrames,
    draws,
    gpu,
    colors: getAndroidMaterialSnapshot().colors,
    textRight: text.right,
    canvasLeft: decoration.left,
    text: copy.textContent,
    textOpacity: getComputedStyle(copy).opacity,
    textColor: getComputedStyle(copy).color,
    background: getComputedStyle(surface).backgroundColor,
    viewportWidth: innerWidth,
    x: rect.x,
    y: rect.y,
    width: canvas.width,
    height: canvas.height,
    hidden: document.hidden,
    fallbackDisplay: getComputedStyle(surface.querySelector('.android-liquid-fallback')!).display,
  };
}

export function loseLiquidProbeContext(lost: boolean) {
  const extension = lost
    ? canvas?.getContext('webgl')?.getExtension('WEBGL_lose_context')
    : lossExtension;
  if (!extension) throw new Error('Context loss extension unavailable; cannot accept this lane');
  if (lost) {
    lossExtension = extension;
    extension.loseContext();
  } else extension.restoreContext();
}

export function pokeReducedLiquidProbe() {
  canvas?.dispatchEvent(new PointerEvent('pointermove', { clientX: 1, clientY: 1 }));
}
