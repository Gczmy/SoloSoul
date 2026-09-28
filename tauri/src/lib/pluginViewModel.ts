/** 插件任意 JSON 与内建展示模型的边界；wire 类型唯一来源为 generated。 */
import type { PluginEvent, PluginLogLine, PluginResult } from './generated/ipcContracts';

type RequiredNonNullable<T> = { [K in keyof T]-?: NonNullable<T[K]> };
export type ConsentRequestEvent = Pick<PluginEvent, 'eventType'> & {
  eventType: 'consent_request';
} & RequiredNonNullable<
    Pick<
      PluginEvent,
      'requestId' | 'pluginId' | 'pluginName' | 'fieldId' | 'fieldLabel' | 'sensitivityLevel'
    >
  >;
export type DialogRequestEvent = Pick<PluginEvent, 'eventType' | 'jsonData'> & {
  eventType: 'dialog_request';
} & RequiredNonNullable<Pick<PluginEvent, 'requestId' | 'pluginId' | 'pluginName'>>;
export type PluginLogView = Omit<PluginLogLine, 'level'> & {
  level: 'debug' | 'info' | 'warn' | 'error';
};

export type WatermarkResultItem = {
  objectId: string;
  attachmentId: string;
  fileName: string;
  mimeType: string;
  outputPath: string;
};
type WatermarkResultPayload = {
  type: 'watermark_result';
  outputDir: string;
  items: WatermarkResultItem[];
};
type ExpiryGuardianItem = {
  objectId: string;
  objectName: string;
  kind: string;
  expiryDate: string;
  daysRemaining: number;
  urgency: 'expired' | 'critical' | 'warning' | 'notice' | 'safe';
};
type ExpiryGuardianSummary = {
  total: number;
  expired: number;
  critical: number;
  warning: number;
  notice: number;
  safe: number;
};
type ExpiryGuardianPayload = {
  type: 'expiry_guardian';
  title: string;
  locale: string;
  items: ExpiryGuardianItem[];
  summary: ExpiryGuardianSummary;
};
export type PluginDisplayResult =
  | { type: 'text'; content: string }
  | {
      type: 'key_value';
      title: string;
      pairs: Array<{ key: string; value: string; tag?: string; tagCode?: string }>;
    }
  | { type: 'table'; headers: string[]; rows: string[][] }
  | { type: 'markdown'; content: string }
  | WatermarkResultPayload
  | ExpiryGuardianPayload;

export interface DialogConfig {
  type: 'alert' | 'confirm' | 'radio_list' | 'checkbox_list' | 'input';
  title?: string;
  message?: string;
  items?: Array<{ id: string; label: string }>;
  defaultValue?: string;
  placeholder?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}
function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item: unknown) => typeof item === 'string');
}
function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}
function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || typeof value === 'string';
}

export function isPluginLogLine(value: unknown): value is PluginLogView {
  return (
    isRecord(value) &&
    typeof value.id === 'string' &&
    typeof value.message === 'string' &&
    isFiniteNumber(value.timestamp) &&
    (value.level === 'debug' ||
      value.level === 'info' ||
      value.level === 'warn' ||
      value.level === 'error')
  );
}

export function isPluginDisplayResult(value: unknown): value is PluginDisplayResult {
  if (!isRecord(value)) return false;
  switch (value.type) {
    case 'text':
    case 'markdown':
      return typeof value.content === 'string';
    case 'key_value':
      return (
        typeof value.title === 'string' &&
        Array.isArray(value.pairs) &&
        value.pairs.every(
          (pair: unknown) =>
            isRecord(pair) &&
            typeof pair.key === 'string' &&
            typeof pair.value === 'string' &&
            isOptionalString(pair.tag) &&
            isOptionalString(pair.tagCode),
        )
      );
    case 'table':
      return (
        isStringArray(value.headers) && Array.isArray(value.rows) && value.rows.every(isStringArray)
      );
    case 'watermark_result':
      return (
        typeof value.outputDir === 'string' &&
        Array.isArray(value.items) &&
        value.items.every(
          (item: unknown) =>
            isRecord(item) &&
            typeof item.objectId === 'string' &&
            typeof item.attachmentId === 'string' &&
            typeof item.fileName === 'string' &&
            typeof item.mimeType === 'string' &&
            typeof item.outputPath === 'string',
        )
      );
    case 'expiry_guardian': {
      const summary = value.summary;
      return (
        typeof value.title === 'string' &&
        typeof value.locale === 'string' &&
        Array.isArray(value.items) &&
        value.items.every(
          (item: unknown) =>
            isRecord(item) &&
            typeof item.objectId === 'string' &&
            typeof item.objectName === 'string' &&
            typeof item.kind === 'string' &&
            typeof item.expiryDate === 'string' &&
            isFiniteNumber(item.daysRemaining) &&
            (item.urgency === 'expired' ||
              item.urgency === 'critical' ||
              item.urgency === 'warning' ||
              item.urgency === 'notice' ||
              item.urgency === 'safe'),
        ) &&
        isRecord(summary) &&
        isFiniteNumber(summary.total) &&
        isFiniteNumber(summary.expired) &&
        isFiniteNumber(summary.critical) &&
        isFiniteNumber(summary.warning) &&
        isFiniteNumber(summary.notice) &&
        isFiniteNumber(summary.safe)
      );
    }
    default:
      return false;
  }
}

export function isConsentRequestEvent(value: unknown): value is ConsentRequestEvent {
  return (
    isRecord(value) &&
    value.eventType === 'consent_request' &&
    isNonEmptyString(value.requestId) &&
    isNonEmptyString(value.pluginId) &&
    typeof value.pluginName === 'string' &&
    isNonEmptyString(value.fieldId) &&
    typeof value.fieldLabel === 'string' &&
    typeof value.sensitivityLevel === 'string'
  );
}

export function isDialogRequestEvent(value: unknown): value is DialogRequestEvent {
  return (
    isRecord(value) &&
    value.eventType === 'dialog_request' &&
    isNonEmptyString(value.requestId) &&
    isNonEmptyString(value.pluginId) &&
    typeof value.pluginName === 'string' &&
    typeof value.jsonData === 'string'
  );
}

export function isPluginCompletedEvent(value: unknown): value is Pick<PluginResult, 'exitCode'> {
  return isRecord(value) && isFiniteNumber(value.exitCode);
}
