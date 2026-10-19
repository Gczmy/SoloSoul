import type { ThemeConfig } from '@/types';
import { applyTheme, getSystemTheme, listenForSystemTheme } from './theme';
import { withTimeout } from './withTimeout';

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
  private listening = false;
  private systemReady: Promise<void> | undefined;
  private startup: { config: ThemeConfig; isCurrent: () => boolean } | undefined;

  constructor(private readonly sources: ThemeControllerSources) {}

  private report(error: unknown) {
    this.sources.onError?.(error);
  }

  private listenToSystem(): Promise<void> {
    if (this.systemReady) return this.systemReady;
    this.listening = true;
    const lifetime = ++this.lifetime;
    let notifications = 0;
    try {
      const listening = (this.sources.listenSystem ?? listenForSystemTheme)((mode) => {
        if (!this.listening || this.lifetime !== lifetime) return;
        notifications += 1;
        if (!this.connected) {
          const startup = this.startup;
          if (startup?.config.preset === 'system' && startup.isCurrent()) {
            void this.applyStartup(
              { ...startup.config, resolvedSystemTheme: mode },
              startup.isCurrent,
            );
          }
        } else if (this.active && this.owners && this.sources.read().config.preset === 'system') {
          void this.refresh(mode);
        }
      });
      this.systemReady = listening.then(
        (dispose) => {
          if (!this.listening || this.lifetime !== lifetime) {
            dispose();
            return;
          }
          this.disposeSystem = dispose;
          // 注册前的一次查询可能已过时。已收到通知时以该通知为准，避免旧查询回跳。
          if (
            this.active &&
            this.owners &&
            !notifications &&
            this.sources.read().config.preset === 'system'
          ) {
            void this.refresh(undefined, true);
          }
        },
        (error) => {
          if (this.lifetime === lifetime) {
            this.listening = false;
            this.systemReady = undefined;
            this.report(error);
          }
        },
      );
    } catch (error) {
      this.listening = false;
      this.report(error);
      return Promise.resolve();
    }
    return this.systemReady;
  }

  /** React 前的缓存首帧使用同一通道；第一次交接后不再允许启动任务回填。 */
  async applyStartup(config: ThemeConfig, isCurrent: () => boolean = () => true) {
    if (this.connected) return;
    this.startup = { config, isCurrent };
    const generation = ++this.generation;
    const guard = () => !this.connected && generation === this.generation && isCurrent();
    try {
      const ready = this.listenToSystem();
      await withTimeout(ready, 600).catch(() => {
        // 原生监听迟到不能永久阻塞启动；交接前完成则补读，交接后由 refresh 补读。
        void ready.then(() => {
          if (guard())
            void this.applyStartup({ ...config, resolvedSystemTheme: undefined }, isCurrent);
        });
      });
      if (!guard()) return;
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
    this.startup = undefined;
    if (!this.active) {
      this.active = true;
      this.lastKey = undefined;
      this.disposeSource = this.sources.subscribe(() => void this.refresh());
      void this.listenToSystem();
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
        this.listening = false;
        this.systemReady = undefined;
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
