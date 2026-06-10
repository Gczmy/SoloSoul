import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

export type TemplateSourceStatus = 'system' | 'active' | 'soft_deleted' | 'permanently_deleted' | 'manual';

export interface SensitivityMapData {
  version: number;
  entries: Record<string, SensitivityLevel>;
  template_sources: Record<string, string | null>;
  template_source_statuses: Record<string, TemplateSourceStatus>;
  last_modified_at: string;
}

export interface SensitivityLogEntry {
  timestamp: string;
  field_id: string;
  old_level: SensitivityLevel;
  new_level: SensitivityLevel;
  reason: string;
}

interface SensitivityState {
  map: SensitivityMapData | null;
  log: SensitivityLogEntry[];
  isLoading: boolean;
  error: string | null;

  loadMap: () => Promise<void>;
  getFieldLevel: (fieldId: string) => Promise<SensitivityLevel>;
  updateField: (fieldId: string, newLevel: SensitivityLevel, password: string, reason?: string) => Promise<void>;
  loadLog: (limit?: number) => Promise<void>;
}

export const useSensitivityStore = create<SensitivityState>((set) => ({
  map: null,
  log: [],
  isLoading: false,
  error: null,

  loadMap: async () => {
    set({ isLoading: true, error: null });
    try {
      const map = await invoke<SensitivityMapData>('sensitivity_get_map');
      set({ map, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  getFieldLevel: async (fieldId) => {
    const level = await invoke<string>('sensitivity_get_field', { fieldId });
    return level as SensitivityLevel;
  },

  updateField: async (fieldId, newLevel, password, reason) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('sensitivity_update_field', { fieldId, newLevel, password, reason: reason || null });
      await set({ isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  loadLog: async (limit = 100) => {
    try {
      const log = await invoke<SensitivityLogEntry[]>('sensitivity_get_log', { limit });
      set({ log });
    } catch {
      // silent
    }
  },
}));
