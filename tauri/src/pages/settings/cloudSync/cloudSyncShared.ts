/**
 * P007：云同步设置页共享类型与常量（CloudSyncPage 拆分产物）。
 * 页面主组件与各 section 子组件共同引用，避免循环依赖。
 */

export interface RetentionPolicy {
  recentFull: number;
  daily: boolean;
  weekly: boolean;
  monthly: boolean;
}

export const CONNECTOR_OPTIONS = [
  { value: 'webdav', label: 'WebDAV (坚果云 / Nextcloud / Alist / 自建)' },
] as const;

export const DEFAULT_RETENTION: RetentionPolicy = {
  recentFull: 10,
  daily: true,
  weekly: true,
  monthly: true,
};

export const DEFAULT_WEBDAV_CONFIG: Record<string, unknown> = {
  baseUrl: 'https://dav.jianguoyun.com/dav/',
  username: '',
  password: '',
  rootPrefix: '/SoloSoul/',
};

/** 已保存配置的形状（cloud_sync_get_config 返回 + 前端表单快照）。 */
export interface SavedCloudSyncConfig {
  connectorType: string;
  configJson: Record<string, unknown>;
  enabled: boolean;
  intervalSecs: number;
  wifiOnly: boolean;
  retention: RetentionPolicy;
  lastSyncAt?: string;
}
