import { create } from 'zustand';
import type { ReactNode } from 'react';
import type { Location } from 'react-router-dom';
import { onRequestSessionChange } from '@/lib/sessionRequests';

/** 页面配置由常驻 ShellLayout 消费，页面只拥有自己的当前注册。 */
export interface ShellConfig {
  title: string;
  actions?: ReactNode;
  primaryActions?: ReactNode;
  onBack?: () => void;
  objectInspector?: ObjectInspectorContext;
}

/** 仅登记当前对象身份；正文和附件不进入全局布局状态。 */
export interface ObjectInspectorContext {
  objectId: string;
  objectName: string;
  typeId?: string;
  ready: boolean;
  failed?: boolean;
}

export const EMPTY_SHELL_CONFIG: Readonly<ShellConfig> = {
  title: '',
  actions: undefined,
  primaryActions: undefined,
  onBack: undefined,
  objectInspector: undefined,
};

export interface ShellRegistration {
  readonly owner: symbol;
  readonly route: string;
  readonly isCurrent: () => boolean;
  /** 同一页面内部参数切换时保留控件节点；交接期间必须 inert。 */
  readonly continuityScope?: string;
}

/** 同路径重新导航、动态参数、查询和锚点变化均是新的路由身份。 */
export function shellRouteIdentity(
  location: Pick<Location, 'key' | 'pathname' | 'search' | 'hash'>,
) {
  return JSON.stringify([location.key, location.pathname, location.search, location.hash]);
}

interface ShellConfigState extends ShellConfig {
  registration: ShellRegistration | null;
  register: (registration: ShellRegistration, config: ShellConfig) => void;
  unregister: (registration: ShellRegistration) => void;
}

export const useShellConfigStore = create<ShellConfigState>((set) => ({
  ...EMPTY_SHELL_CONFIG,
  registration: null,
  register: (registration, config) =>
    set((prev) => {
      // 旧页面重渲染也不能获取新会话权限，恢复已释放的节点或闭包。
      if (!registration.isCurrent()) return prev;
      if (
        prev.registration === registration &&
        prev.title === config.title &&
        prev.actions === config.actions &&
        prev.primaryActions === config.primaryActions &&
        prev.onBack === config.onBack &&
        prev.objectInspector === config.objectInspector
      ) {
        return prev;
      }
      return {
        registration,
        title: config.title,
        actions: config.actions,
        primaryActions: config.primaryActions,
        onBack: config.onBack,
        objectInspector: config.objectInspector,
      };
    }),
  unregister: (registration) =>
    set((prev) =>
      prev.registration === registration ? { ...EMPTY_SHELL_CONFIG, registration: null } : prev,
    ),
}));

onRequestSessionChange(() => {
  useShellConfigStore.setState({ ...EMPTY_SHELL_CONFIG, registration: null });
});
