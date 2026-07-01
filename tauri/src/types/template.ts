/**
 * Template system types (§29 模板系统重构 P1)
 * Mirrors solosoul-vault/src/lib.rs PropertyType / TemplateProperty / UserTemplate
 */

export type PropertyType =
  | 'text'
  | 'multiline'
  | 'number'
  | 'date'
  | 'datetime'
  | 'boolean'
  | 'select'
  | 'multiselect'
  | 'url'
  | 'email'
  | 'phone'
  | 'file';

export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

export interface ContractRoleBinding {
  contractTypeId: string;
  roleId: string;
}

export interface TemplateProperty {
  id: string;
  name: string;
  type: PropertyType;
  /** Sensitivity level (4-tier). Replaces legacy `sensitive` boolean. */
  sensitivityLevel?: SensitivityLevel;
  /** Legacy boolean — kept for backward compat. */
  sensitive?: boolean;
  options?: string[]; // for select / multiselect
  /** ISO 8601 timestamp; if set, the field is soft-deleted but retained for old objects. */
  deprecatedAt?: string;
  /** 插件合约字段映射 — 当此属性映射到插件合约中的字段时为 true。 */
  contractField?: boolean;
  /** 新版绑定：一个字段可绑定到多个插件契约的多个角色。 */
  contractBindings?: ContractRoleBinding[];
}

export interface UserTemplate {
  id: string;
  accountId: string;
  name: string;
  iconId?: string;
  properties: TemplateProperty[];
  category?: string;
  createdAt: string;
  updatedAt?: string;
  /** 插件合约类型 ID — 绑定到插件合约的模板类型标识。 */
  contractTypeId?: string;
}

/** Frontend-only helper: map from backend snake_case to frontend camelCase */
export interface UserTemplateRaw {
  id: string;
  account_id: string;
  name: string;
  icon_id?: string;
  properties: TemplateProperty[];
  category?: string;
  created_at: string;
  updated_at?: string;
}
