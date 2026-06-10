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

export interface TemplateProperty {
  id: string;
  name: string;
  type: PropertyType;
  /** Sensitivity level (4-tier). Replaces legacy `sensitive` boolean. */
  sensitivityLevel?: SensitivityLevel;
  /** Legacy boolean — kept for backward compat. */
  sensitive?: boolean;
  options?: string[]; // for select / multiselect
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
