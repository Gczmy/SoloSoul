import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { usePluginStore } from '@/stores/pluginStore';
import { resolvePluginName, deriveContractBindings, type PluginManifest } from '@/lib/plugin';
import type { ContractRoleBinding, TemplateProperty } from '@/types/template';
import type { FlattenedContract } from './TemplateFieldBindingSection';
import { logger } from '@/lib/logger';

export interface UseTemplateEditorStateResult {
  showIconPicker: boolean;
  setShowIconPicker: React.Dispatch<React.SetStateAction<boolean>>;
  expandedBindingFields: Set<string>;
  selectedContractId: Record<string, string>;
  setSelectedContractId: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  selectedRoleId: Record<string, string>;
  setSelectedRoleId: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  installedPlugins: PluginManifest[];
  flattenContracts: FlattenedContract[];
  toggleBindingExpanded: (fieldKey: string, fieldIdx: number) => void;
}

/**
 * 模板编辑器的本地 UI 状态：图标选择器开关、插件绑定展开/契约/角色选择，
 * 以及插件列表加载与契约展平。受控 props 之外的全部状态。
 */
export function useTemplateEditorState(
  editProperties: TemplateProperty[],
  editContractTypeId: string,
  onUpdatePropertyContractBindings: (index: number, bindings: ContractRoleBinding[]) => void,
): UseTemplateEditorStateResult {
  const [showIconPicker, setShowIconPicker] = useState(false);

  // 插件绑定 UI 状态
  const [expandedBindingFields, setExpandedBindingFields] = useState<Set<string>>(new Set());
  const [selectedContractId, setSelectedContractId] = useState<Record<string, string>>({});
  const [selectedRoleId, setSelectedRoleId] = useState<Record<string, string>>({});

  const installedPlugins = usePluginStore((s) => s.installedPlugins);
  const loadInstalled = usePluginStore((s) => s.loadInstalled);
  const { i18n } = useTranslation(['settings', 'common', 'editor']);

  // 加载已安装插件列表（用于展示契约角色）
  React.useEffect(() => {
    if (installedPlugins.length === 0) {
      // P042: 插件列表加载失败不再静默吞错（降级表现为契约角色绑定缺失，需可诊断）。
      loadInstalled().catch((err) =>
        logger.warn('[TemplateEditor] Load installed plugins failed:', err),
      );
    }
  }, [installedPlugins.length, loadInstalled]);

  // 将已安装插件的所有契约展平为一个列表
  const flattenContracts = React.useMemo<FlattenedContract[]>(() => {
    const currentLocale = i18n.language || 'zh-CN';
    const list: FlattenedContract[] = [];
    for (const plugin of installedPlugins) {
      for (const contract of plugin.contracts || []) {
        if (contract.roles && contract.roles.length > 0) {
          list.push({
            pluginId: plugin.id,
            pluginName: resolvePluginName(plugin, currentLocale),
            contract,
          });
        }
      }
    }
    return list;
  }, [installedPlugins, i18n.language]);

  const toggleBindingExpanded = (fieldKey: string, fieldIdx: number) => {
    const willExpand = !expandedBindingFields.has(fieldKey);
    setExpandedBindingFields((prev) => {
      const next = new Set(prev);
      if (next.has(fieldKey)) {
        next.delete(fieldKey);
      } else {
        next.add(fieldKey);
      }
      return next;
    });
    // 展开时自动推导并持久化 contractField: true 但无硬编码 bindings 的字段
    if (willExpand) {
      const prop = editProperties[fieldIdx];
      if (prop) {
        const existingBindings = prop.contractBindings || [];
        if (existingBindings.length === 0 && prop.contractField && editContractTypeId) {
          const derived = deriveContractBindings(editContractTypeId, prop.id, installedPlugins);
          if (derived.length > 0) {
            onUpdatePropertyContractBindings(fieldIdx, derived);
          }
        }
      }
    }
  };

  return {
    showIconPicker,
    setShowIconPicker,
    expandedBindingFields,
    selectedContractId,
    setSelectedContractId,
    selectedRoleId,
    setSelectedRoleId,
    installedPlugins,
    flattenContracts,
    toggleBindingExpanded,
  };
}
