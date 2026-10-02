import type { ThemeConfig } from '@/types';
import { applyTheme, getSystemTheme, listenForSystemTheme } from './theme';

type Mode = 'light' | 'dark';
type Dispose = () => void;

/** context 包含账户/会话及影响原生材质的有效设置版本，由应用绑定提供。 */
export interface ThemeSnapshot {
  config: ThemeConfig;
  context: string;
}

export interface ThemeControllerSources {
  read: () => ThemeSnapshot;
  subscribe: (refresh: () => void) => Dispose;
  resolveSystem?: () => Promise<Mode>;
  listenSystem?: (notify: (mode: Mode) => void) => Promise<Dispose>;
  apply?: (config: ThemeConfig, isCurrent: () => boolean) => Promise<void>;
  onError?: (error: unknown) => void;
}

const keyOf = (snapshot: ThemeSnapshot) => JSON.stringify(snapshot);

/** 单一主题请求通道：解析可以并发，但过期结果不能交付 DOM 或原生队列。 */
export class ThemeController {
  private owners = 0;
  private active = false;
  private connected = false;
  private generation = 0;
  private lifetime = 0;
  private stopRevision = 0;
  private lastKey: string | undefined;
  private disposeSource: Dispose | undefined;
  private disposeSystem: Dispose | undefined;

  constructor(private readonly sources: ThemeControllerSources) {}

  private report(error: unknown) {
    this.sources.onError?.(error);
  }

  /** React 前的缓存首帧使用同一通道；第一次交接后不再允许启动任务回填。 */
  async applyStartup(config: ThemeConfig, isCurrent: () => boolean = () => true) {
    if (this.connected) return;
    const generation = ++this.generation;
    const guard = () => !this.connected && generation === this.generation && isCurrent();
    try {
      await (this.sources.apply ?? applyTheme)(config, guard);
    } catch (error) {
      if (guard()) this.report(error);
    }
  }

  acquire(): Dispose {
    const wasIdle = this.owners === 0;
    this.owners += 1;
    this.stopRevision += 1;
    this.connected = true;
    if (!this.active) {
      this.active = true;
      const lifetime = ++this.lifetime;
      this.lastKey = undefined;
      this.disposeSource = this.sources.subscribe(() => void this.refresh());
      try {
        const listening = (this.sources.listenSystem ?? listenForSystemTheme)((mode) => {
          if (!this.active || !this.owners || this.lifetime !== lifetime) return;
          if (this.sources.read().config.preset === 'system') void this.refresh(mode);
        });
        void listening.then(
          (dispose) => {
            if (!this.active || this.lifetime !== lifetime) dispose();
            else this.disposeSystem = dispose;
          },
          (error) => {
            if (this.active && this.lifetime === lifetime) this.report(error);
          },
        );
      } catch (error) {
        this.report(error);
      }
    }
    void this.refresh(undefined, wasIdle);
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.owners -= 1;
      if (this.owners) return;
      // 立即撤销旧请求；仅监听释放推迟到微任务，以吸收 StrictMode setup/cleanup/setup。
      this.generation += 1;
      const revision = ++this.stopRevision;
      queueMicrotask(() => {
        if (this.owners || this.stopRevision !== revision) return;
        this.active = false;
        this.lifetime += 1;
        this.disposeSource?.();
        this.disposeSystem?.();
        this.disposeSource = undefined;
        this.disposeSystem = undefined;
        this.lastKey = undefined;
      });
    };
  }

  async refresh(resolvedSystemTheme?: Mode, force = false): Promise<void> {
    if (!this.active || !this.owners) return;
    const generation = this.generation + 1;
    let key: string | undefined;
    const guard = () => {
      if (!this.active || !this.owners || this.generation !== generation) return false;
      try {
        return keyOf(this.sources.read()) === key;
      } catch {
        return false;
      }
    };
    try {
      const snapshot = this.sources.read();
      key = keyOf(snapshot);
      if (!force && resolvedSystemTheme === undefined && this.lastKey === key) return;
      this.generation = generation;
      this.lastKey = key;
      const config = { ...snapshot.config };
      const mode =
        config.preset === 'system'
          ? (resolvedSystemTheme ?? (await (this.sources.resolveSystem ?? getSystemTheme)()))
          : undefined;
      if (!guard()) return;
      await (this.sources.apply ?? applyTheme)({ ...config, resolvedSystemTheme: mode }, guard);
    } catch (error) {
      if (guard()) {
        this.lastKey = undefined;
        this.report(error);
      }
    }
  }
}

// 启动缓存与 React 生命周期共用实例；绑定模块只提供 Store 读取/订阅，不应用主题。
let applicationSources: ThemeControllerSources | undefined;
export function bindThemeController(sources: ThemeControllerSources): void {
  applicationSources ??= sources;
}
export const themeController = new ThemeController({
  read: () => {
    if (!applicationSources) throw new Error('ThemeController is not bound');
    return applicationSources.read();
  },
  subscribe: (refresh) => {
    if (!applicationSources) throw new Error('ThemeController is not bound');
    return applicationSources.subscribe(refresh);
  },
  onError: (error) => applicationSources?.onError?.(error),
});
