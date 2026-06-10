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

export interface TemplateProperty {
  id: string;
  name: string;
  type: PropertyType;
  sensitive?: boolean;
  options?: string[]; // for select / multiselect
}

export interface UserTemplate {
  id: string;
  accountId: string;
  name: string;
  iconId?: string;
  properties: TemplateProperty[];
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
  created_at: string;
  updated_at?: string;
}
