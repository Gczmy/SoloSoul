import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';
import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { toObjectDataView } from '@/lib/objectViewModel';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { logger } from '@/lib/logger';
import { useObjectStore, type ObjectData, type ObjectSummary } from '@/stores/objectStore';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { useObjectDetailVerification } from './useObjectDetailVerification';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { resolveCollectionLabel } from '@/lib/utils';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { useSettingsStore } from '@/stores/settingsStore';
import type { TemplateProperty } from '@/types/template';
import { useDragToAttach } from '@/hooks/useDragToAttach';
import { isAndroidSync, isMobilePlatformSync } from '@/lib/platform';
import {
  flattenPropertiesGrouped,
  buildDetailGuidePages,
} from '@/components/object/objectDetailUtils';

export interface ObjectDetailModalProps {
  /** 已加载的对象摘要/完整数据。与 objectId 二选一，优先使用此值。 */
  object?: ObjectSummary | ObjectData;
  /** 若未提供 object，则通过 objectId 自动拉取完整对象数据。 */
  objectId?: string;
  /** 模板已更新且对象尚未同步时显示提示条。 */
  needsSync?: boolean;
  onClose: () => void;
  onHistory?: () => void;
  onAttachments?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  /** 用户确认应用模板更新。 */
  onSyncTemplate?: () => void;
  /** 用户选择暂不应用模板更新。 */
  onDismissSync?: () => void;
  /** 查看已归档历史字段。 */
  onViewDeprecatedFields?: () => void;
  /** 附件增删后的回调，用于外部（workspace）刷新计数 badge */
  onAttachmentsChange?: () => void;
}

/**
 * 对象详情弹窗的全部编排逻辑（P046 拆分：数据 hook；W001-④ 再拆后为组合层）。
 * 完整对象拉取（P020 防陈旧）、模板字段解析、敏感度/历史字段推导、复制反馈、
 * 删除流程、历史/附件子视图开关、拖拽上传保留于此；关键字段验证
 * （密码/生物识别/PIN + 揭示状态 + 访问日志）收敛于 useObjectDetailVerification。
 * ObjectDetailModal 组件退化为纯展示组合层。
 */
export function useObjectDetailModal(props: ObjectDetailModalProps) {
  const { object, objectId, onClose, onDelete, onAttachmentsChange } = props;

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  // 摘要不含 accountId：同一 prop 跨账户继续存在时，不再把它当作新账户的数据。
  const objectSourceRef = useRef({ object, accountId });
  useEffect(() => {
    if (objectSourceRef.current.object !== object) {
      objectSourceRef.current = { object, accountId };
    }
  }, [object, accountId]);
  const sourceMatchesAccount =
    objectSourceRef.current.object !== object || objectSourceRef.current.accountId === accountId;
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  // P055: 分字段 selector，避免 store 任何变化触发整页重渲染（函数引用稳定）
  const templates = useTemplateStore((s) => s.templates);
  const loadTemplates = useTemplateStore((s) => s.loadTemplates);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const [fetchedObj, setFetchedObj] = useState<{
    accountId: string;
    objectId: string;
    data: ObjectData;
  } | null>(null);
  // P020 复核：object 可能是截断预览摘要（object_list 仅保留前 8 字段/200 字符），
  // 详情弹窗必须始终拉取完整对象，避免丢字段/值被静默截断。
  // 完整对象带账户归属；账户切换后即使调用方尚未卸载弹窗，也不能沿用旧账户数据。
  const suppliedObject =
    accountId &&
    object &&
    sourceMatchesAccount &&
    (!('accountId' in object) || object.accountId === accountId) &&
    (!objectId || object.id === objectId)
      ? object
      : undefined;
  const objId = objectId ?? suppliedObject?.id;
  const isAndroid = isAndroidSync();
  const [attachmentRevision, setAttachmentRevision] = useState(0);
  const [attachmentStats, setAttachmentStats] = useState<{ objectId: string; count: number }>();
  const attachmentCount = attachmentStats?.objectId === objId ? attachmentStats?.count : undefined;
  const handleAttachmentsChange = useCallback(() => {
    if (isAndroid) setAttachmentRevision((revision) => revision + 1);
    onAttachmentsChange?.();
  }, [isAndroid, onAttachmentsChange]);

  // 只查询计数，不为底栏解密/加载整个附件列表；切换对象时忽略旧请求。
  useEffect(() => {
    if (!isAndroid || !objId || !accountId) return;
    let cancelled = false;
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: [objId] })
      .then((counts) => {
        if (!cancelled) setAttachmentStats({ objectId: objId, count: counts[objId] ?? 0 });
      })
      .catch((error) => logger.warn('[ObjectDetail] Attachment count failed:', error));
    return () => {
      cancelled = true;
    };
  }, [isAndroid, objId, accountId, attachmentRevision]);
  // P020 二次复核：调用方已传入完整 ObjectData（含 accountId，如 ?objectId= 路径
  // 经 object_get 拉取后传入、或模板同步后 refreshDetailObjAfterSync 刷新）时无需
  // 再拉取——完整数据直接可用，避免双重 object_get。ObjectSummary 无 accountId。
  const isCompleteObject = useMemo(
    () => !!suppliedObject && 'accountId' in suppliedObject,
    [suppliedObject],
  );
  const [loading, setLoading] = useState(!suppliedObject && !!objId);
  // P025：复制反馈收敛至共享 hook（按字段名键控）
  const { copy: copyText, copiedKey: copiedField } = useCopyToClipboard(COPY_FEEDBACK_DURATION_MS);
  const fetchIdRef = useRef(0);

  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [showAttachments, setShowAttachments] = useState(false);

  // P020 复核：拉取结果优先于传入摘要——传入 object 仅作过渡展示，
  // 完整数据（fetchedObj）到达后立即升级，保证详情弹窗渲染完整 properties。
  const obj = useMemo(() => {
    if (isCompleteObject) return suppliedObject;
    if (fetchedObj && fetchedObj.accountId === accountId && fetchedObj.objectId === objId) {
      return fetchedObj.data;
    }
    return suppliedObject;
  }, [accountId, fetchedObj, isCompleteObject, objId, suppliedObject]);
  const { ref: detailDragRef, dragState: detailDragState } = useDragToAttach(obj?.id || null, {
    onComplete: handleAttachmentsChange,
  });

  useEffect(() => {
    loadTemplates().catch((err) => logger.warn('[ObjectDetail] Load templates failed:', err));
  }, [loadTemplates]);

  useEffect(() => {
    const id = ++fetchIdRef.current;
    if (!objId || !accountId) {
      setLoading(false);
      return;
    }
    // 完整数据直接使用当前 prop；前一个摘要请求已由请求代次失效。
    if (isCompleteObject && suppliedObject && 'accountId' in suppliedObject) {
      setLoading(false);
      return;
    }
    // 有 object（摘要）时仍保持当前展示，拉取完成后再升级，避免闪屏。
    if (!suppliedObject) setLoading(true);
    // P020 二次复核：绕开全局 store action（getObject 会置全局 isLoading →
    // 打开弹窗瞬间背后工作区列表被骨架屏替换再换回），直接 invoke object_get；
    // 结果同时写入 currentObjectCache 供其他消费方读取（不置 isLoading）。
    let active = true;
    invokeTypedCommand('object_get', { accountId: accountId, objectId: objId })
      .then((wire) => {
        // Discard stale responses: if a newer fetch started while this one
        // was in-flight, don't overwrite fetchedObj with potentially stale data.
        if (!active || fetchIdRef.current !== id) return;
        const obj = wire === null ? null : toObjectDataView(wire);
        if (obj) {
          useObjectStore.setState((s) => ({
            currentObjectCache: { ...s.currentObjectCache, [objId]: obj },
          }));
        }
        setFetchedObj(obj ? { accountId, objectId: objId, data: obj } : null);
      })
      .catch(() => {
        if (active && fetchIdRef.current === id) setFetchedObj(null);
      })
      .finally(() => {
        if (active && fetchIdRef.current === id) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [suppliedObject, isCompleteObject, objId, accountId]);

  const resolveCollectionLabelLocal = useCallback(
    (typeId: string) => resolveCollectionLabel(typeId, customPages, t),
    [customPages, t],
  );

  // 关键数据验证域（密码/生物识别/PIN 验证对话框 + 揭示状态 + 访问日志 + 探测）
  // 收敛于 useObjectDetailVerification（W001-④ 拆分）。
  const verification = useObjectDetailVerification({
    accountId,
    obj: obj ?? null,
    resolveCollectionLabelLocal,
  });

  // F012: cache the current object's template field map for O(1) lookups.
  const fieldMap = useMemo(() => {
    const tpl = templates.find((t) => t.id === obj?.templateId);
    return new Map<string, TemplateProperty>(tpl?.properties.map((p) => [p.id, p]) ?? []);
  }, [templates, obj?.templateId]);

  // 从 properties 中提取 __fields（即使模板被删除，字段定义仍保留在对象上）
  const objFieldDefs = useMemo(() => {
    const raw = (obj?.properties as Record<string, unknown>)?.__fields;
    return raw as
      | Record<string, { name: string; type: string; sensitivityLevel?: unknown }>
      | undefined;
  }, [obj?.properties]);

  // 已归档的历史字段（模板字段类型不兼容变更时产生）
  const deprecatedFields = useMemo(() => {
    const raw = (obj?.properties as Record<string, unknown>)?.__deprecatedFields;
    return Array.isArray(raw) ? raw : [];
  }, [obj?.properties]);

  const getFieldProperty = (fieldKey: string): TemplateProperty | undefined => {
    return fieldMap.get(fieldKey);
  };

  const getFieldSensitivity = (fieldKey: string): SensitivityLevel => {
    return resolveFieldSensitivity({
      fieldId: fieldKey,
      propertyLabels: obj?.propertyLabels,
      definition: objFieldDefs?.[fieldKey],
      template: getFieldProperty(fieldKey),
    });
  };
  const isFieldDeprecated = (fieldKey: string): boolean => {
    return !!getFieldProperty(fieldKey)?.deprecatedAt;
  };

  const getFieldName = (fieldKey: string, label?: string): string => {
    if (label) return label;
    return getFieldProperty(fieldKey)?.name || objFieldDefs?.[fieldKey]?.name || fieldKey;
  };

  const handleCopy = async (value: string, key: string) => {
    await copyText(value, key);
  };

  const handleDelete = async () => {
    if (onDelete) {
      onDelete();
      return;
    }
    if (!obj) return;
    setDeleting(true);
    try {
      await useObjectStore.getState().deleteObject(obj.id);
      onClose();
      setConfirmDelete(false);
    } catch (err) {
      // P006: 删除失败不得静默——保持确认弹窗打开并提示，避免用户
      // 以为删除成功而对象仍存在。
      import('@/stores/uiStore').then(({ useUiStore }) => {
        useUiStore.getState().showToast({
          type: 'error',
          message: `${t('common:delete_failed')}: ${err}`, // 具体错误便于诊断
          duration: 5000,
        });
      });
      logger.warn('[ObjectDetail] Delete object failed:', err);
    } finally {
      setDeleting(false);
    }
  };

  const detailTpl = obj?.templateId ? templates.find((t) => t.id === obj.templateId) : undefined;
  // 模板匹配需同时满足 ID 和页面归属（与编辑器 ObjectEditorPage 对齐）。
  // !! 归一化为 boolean：原实现仅用于三元/条件真值判断，语义等价。
  const detailTplMatch = !!detailTpl && (detailTpl.category || 'identity') === obj?.typeId;
  const ObjectDetailIcon = detailTpl?.iconId
    ? resolveCustomIcon(detailTpl.iconId)
    : PAGE_ICON_MAP.custom;
  // P019: fieldOrder / fields 每次渲染重算改为 useMemo（与 HistoryViewer /
  // WorkspaceObjectCard 已确立的 memo 化范式一致）。
  const fieldOrder = useMemo(
    () => templates.find((t) => t.id === obj?.templateId)?.properties.map((p) => p.id),
    [templates, obj?.templateId],
  );
  // 详情卡片：动态字段组保留树状结构（组头 + 子行），与历史快照渲染同构
  const fields = useMemo(
    () => flattenPropertiesGrouped(obj?.properties, fieldOrder, objFieldDefs),
    [obj?.properties, fieldOrder, objFieldDefs],
  );

  const isMobilePlatform = isMobilePlatformSync();

  const detailGuidePages = useMemo(
    () => buildDetailGuidePages(t, isMobilePlatform, isAndroid),
    [t, isMobilePlatform, isAndroid],
  );

  return {
    t,
    accountId,
    // 数据
    loading,
    obj,
    attachmentCount,
    handleAttachmentsChange,
    objFieldDefs,
    fields,
    fieldOrder,
    deprecatedFields,
    detailTpl,
    detailTplMatch,
    ObjectDetailIcon,
    resolveCollectionLabelLocal,
    detailGuidePages,
    // 敏感度/字段解析
    getFieldProperty,
    getFieldSensitivity,
    isFieldDeprecated,
    getFieldName,
    // 揭示/复制
    handleCopy,
    copiedField,
    // 删除流程
    confirmDelete,
    setConfirmDelete,
    deleting,
    handleDelete,
    // 历史/附件开关
    showHistory,
    setShowHistory,
    showAttachments,
    setShowAttachments,
    // 关键数据验证（useObjectDetailVerification 收敛）
    ...verification,
    // 拖拽上传
    detailDragRef,
    detailDragState,
  };
}
