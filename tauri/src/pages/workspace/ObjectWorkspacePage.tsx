import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import buttonStyles from '@/components/ui/Button.module.css';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, type ObjectSummary, type ObjectData } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import type { TemplateProperty } from '@/types/template';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { objectNeedsSync, type TemplateSyncResult, type DeprecatedField } from '@/lib/templateSync';

// Labels resolved at render time via t() so they support i18n
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { TemplateSyncConfirmDialog } from '@/components/object/TemplateSyncConfirmDialog';
import { DeprecatedFieldsViewer } from '@/components/object/DeprecatedFieldsViewer';
import {
  Trash,
  Search,
  LayoutList,
  Maximize2,
  Paperclip,
  Upload,
  LayoutTemplate,
  Shield,
  Pencil,
  Trash2,
  FileText,
  Settings,
} from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';

import { WorkspaceObjectCard } from './WorkspaceObjectCard';
import { WorkspaceCategoryTabs } from '@/components/workspace/WorkspaceCategoryTabs';
import { ConfirmDeleteDialog } from '@/components/workspace/ConfirmDeleteDialog';

import { PageGuide } from '@/components/guide/PageGuide';
import { ICON_SIZE } from '@/lib/constants';
import workspaceStyles from './ObjectWorkspacePage.module.css';

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const detailObjectId = searchParams.get('objectId');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState('');
  const [, setDeletingId] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [confirmPageDelete, setConfirmPageDelete] = useState(false);
  const [historyObj, setHistoryObj] = useState<{
    id: string;
    name: string;
    collectionType: string;
    templateId?: string;
  } | null>(null);
  const [snapshotCounts, setSnapshotCounts] = useState<Record<string, number>>({});
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);
  const [attachmentCounts, setAttachmentCounts] = useState<Record<string, number>>({});
  const [detailObj, setDetailObj] = useState<(ObjectSummary | ObjectData) | null>(null);

  // 模板指纹映射：仅在模板列表变化时异步计算一次，避免切换页面时批量重算导致闪烁。
  const [templateHashMap, setTemplateHashMap] = useState<Map<string, string>>(new Map());

  // 模板同步确认弹窗状态
  const [syncDialog, setSyncDialog] = useState<{
    objectId: string;
    objectName: string;
    result: TemplateSyncResult | null;
    loading: boolean;
  } | null>(null);

  // 忽略模板更新二次确认弹窗状态
  const [dismissConfirm, setDismissConfirm] = useState<{
    objectId: string;
    objectName: string;
    latestHash: string;
  } | null>(null);

  // 历史字段查看器状态
  const [deprecatedViewer, setDeprecatedViewer] = useState<{
    objectId: string;
    objectName: string;
  } | null>(null);
  const [deprecatedFields, setDeprecatedFields] = useState<DeprecatedField[]>([]);

  const accountId = useAuthStore((s) => s.currentAccount?.id);

  // 同步成功后刷新当前打开的详情对象，避免本地 state 保留旧的 templateHash 导致提示条继续显示。
  const refreshDetailObjAfterSync = useCallback(
    async (objectId: string) => {
      if (!accountId || detailObj?.id !== objectId) return;
      try {
        const obj = await invoke<ObjectData | null>('object_get', {
          accountId,
          objectId,
        });
        if (obj) setDetailObj(obj);
      } catch (err) {
        console.warn('[Workspace] Refresh detail object after sync failed:', err);
      }
    },
    [accountId, detailObj?.id],
  );

  // 模板同步确认弹窗打开期间，对应对象的提示条应临时隐藏，避免被弹窗遮罩盖住。
  const [syncDialogOpenForObjectId, setSyncDialogOpenForObjectId] = useState<string | null>(null);

  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const {
    objects,
    loadObjects,
    deleteObject,
    previewSyncTemplate,
    applySyncTemplate,
    ignoreTemplateSync,
    loadDeprecatedFields,
    isLoading,
    error,
  } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const removeCustomPage = useSettingsStore((s) => s.removeCustomPage);
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    loadUserTemplates().catch((err) => console.warn('[Workspace] Load templates failed:', err));
  }, [loadUserTemplates]);

  // Open object detail modal directly when navigated with ?objectId=... (e.g. from search)
  useEffect(() => {
    if (!detailObjectId || !accountId) return;
    invoke('object_get', { objectId: detailObjectId })
      .then((obj) => setDetailObj(obj as (typeof visibleObjects)[number]))
      .catch((err) => console.warn('[Workspace] Fetch object detail failed:', err));
  }, [detailObjectId, accountId]);

  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const resolveCollectionLabel = useCallback(
    (collectionType: string) => {
      if (['identity', 'travel', 'financial', 'professional'].includes(collectionType)) {
        return t(`navigation:${collectionType}`);
      }
      const cp = customPages.find((p) => p.id === collectionType);
      return cp?.name || collectionType;
    },
    [t, customPages],
  );

  const activeCategoryLabel = sectionFilter
    ? t(`navigation:${sectionFilter}`, sectionFilter)
    : null;

  // Inlined from useWorkspacePasswordGuard — shared between detail panel and history viewer.
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<((result: { ok: boolean; method: 'password' | 'touchId' | 'faceId' }) => void) | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({ available: false });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>('biometric_check_availability', { accountId: accountId || '' })
      .then((r) => setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }))
      .catch((err) => console.warn('[Workspace] Biometric check failed:', err));
    if (accountId) {
      invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
        .then((accounts) => {
          const acc = accounts.find((a) => a.id === accountId);
          setPasswordHint(acc?.passwordHint || null);
        })
        .catch(() => { /* ignore */ });
    }
  }, [accountId]);

  const passwordVerify = useCallback(async (): Promise<{ ok: boolean; method: 'password' | 'touchId' | 'faceId' }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  const verifyVaultPassword = useCallback(async (password: string): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('unlock_with_password', { accountId, password });
      return true;
    } catch {
      return false;
    }
  }, [accountId]);

  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', { accountId, location: 'critical_data_access', action: 'unlock', biometryType: bioAvailable.biometryType });
      const method = (bioAvailable.biometryType as 'touchId' | 'faceId') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch {
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  // F011: cache template field metadata so lookups are O(1) instead of O(n²).
  const templateFieldMap = useMemo(() => {
    const map = new Map<string, Map<string, TemplateProperty>>();
    for (const t of userTemplates) {
      map.set(t.id, new Map(t.properties.map((p) => [p.id, p])));
    }
    return map;
  }, [userTemplates]);

  const getFieldProperty = useCallback(
    (templateId: string | undefined, fieldKey: string): TemplateProperty | undefined => {
      return templateFieldMap.get(templateId || '')?.get(fieldKey);
    },
    [templateFieldMap],
  );

  const getFieldSensitivity = useCallback(
    (
      templateId: string | undefined,
      fieldKey: string,
      propertyLabels?: Record<string, string>,
    ): SensitivityLevel => {
      // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
      if (propertyLabels?.[fieldKey]) {
        return propertyLabels[fieldKey] as SensitivityLevel;
      }
      // 2. 回退到模板定义
      return (
        (getFieldProperty(templateId, fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public'
      );
    },
    [getFieldProperty],
  );

  const isFieldDeprecated = useCallback(
    (templateId: string | undefined, fieldKey: string): boolean => {
      return !!getFieldProperty(templateId, fieldKey)?.deprecatedAt;
    },
    [getFieldProperty],
  );

  const getFieldName = useCallback(
    (
      templateId: string | undefined,
      fieldKey: string,
      propertyFields?: Record<string, { name: string }>,
    ): string => {
      return (
        getFieldProperty(templateId, fieldKey)?.name || propertyFields?.[fieldKey]?.name || fieldKey
      );
    },
    [getFieldProperty],
  );

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId, loadObjects]);

  // Debounce searchQuery to avoid high-frequency IPC calls on every keystroke
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearchQuery(searchQuery), DEBOUNCE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const visibleObjects = useMemo(
    () =>
      objects.filter(
        (obj) =>
          obj.collectionType !== 'page' &&
          obj.collectionType !== 'unknown' &&
          obj.name.toLowerCase().includes(debouncedSearchQuery.toLowerCase()),
      ),
    [objects, debouncedSearchQuery],
  );

  const snapshotReqRef = useRef(0);

  // 仅在模板列表变化时计算指纹映射，页面切换时复用，避免同步提示条闪烁。
  useEffect(() => {
    if (!accountId || userTemplates.length === 0) {
      setTemplateHashMap(new Map());
      return;
    }
    let cancelled = false;
    invoke<Record<string, string>>('template_hash_map', { accountId })
      .then((map) => {
        if (cancelled) return;
        setTemplateHashMap(new Map(Object.entries(map)));
      })
      .catch((err) => {
        console.warn('[Workspace] Load template hash map failed:', err);
        if (!cancelled) setTemplateHashMap(new Map());
      });
    return () => {
      cancelled = true;
    };
  }, [accountId, userTemplates]);

  // 同步/忽略模板后主动刷新指纹映射，防止模板列表 state 未变化导致提示条继续显示。
  const refreshTemplateHashMap = useCallback(async () => {
    if (!accountId) return;
    try {
      const map = await invoke<Record<string, string>>('template_hash_map', { accountId });
      setTemplateHashMap(new Map(Object.entries(map)));
    } catch (err) {
      console.warn('[Workspace] Refresh template hash map failed:', err);
    }
  }, [accountId]);

  // Load snapshot counts for visible objects
  useEffect(() => {
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) return;
    const reqId = ++snapshotReqRef.current;
    let mounted = true;
    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then((counts) => {
        if (!mounted || snapshotReqRef.current !== reqId) return; // stale response, discard
        // Ensure every visible object has a snapshot count (default 0)
        const full: Record<string, number> = {};
        for (const id of ids) full[id] = counts[id] ?? 0;
        setSnapshotCounts(full);
      })
      .catch((err) => {
        if (!mounted || snapshotReqRef.current !== reqId) return; // stale error, discard
        console.warn('[Workspace] Snapshot count batch failed:', err);
      });
    return () => {
      mounted = false;
    };
  }, [visibleObjects]);

  // Load attachment counts for visible objects
  const refreshAttachmentCounts = useCallback(() => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) {
      return () => controller.abort();
    }
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: ids })
      .then((counts) => {
        if (!controller.signal.aborted) setAttachmentCounts(counts);
      })
      .catch((err) => console.warn('[Workspace] Attachment count batch failed:', err));
    return () => controller.abort();
  }, [visibleObjects]);

  useEffect(() => {
    return refreshAttachmentCounts();
  }, [refreshAttachmentCounts]);

  const newObjectUrl = pageId
    ? `/editor?parentId=${pageId}`
    : `/editor${sectionFilter ? `?section=${sectionFilter}` : ''}`;

  const handleDelete = async (objectId: string) => {
    setConfirmDelete(null);
    setDeletingId(objectId);
    try {
      await deleteObject(objectId);
    } finally {
      setDeletingId(null);
    }
  };

  const handleStartSync = useCallback(
    async (objectId: string, objectName: string) => {
      if (!accountId) return;
      setSyncDialogOpenForObjectId(objectId);
      setSyncDialog({ objectId, objectName, result: null, loading: true });
      try {
        const result = await previewSyncTemplate(accountId, objectId);
        if (!result.hasChanges) {
          // 无实际字段差异时直接应用同步（仅刷新 template_hash），避免提示条反复出现。
          setSyncDialog(null);
          setSyncDialogOpenForObjectId(null);
          await applySyncTemplate(accountId, objectId);
          if (pageId) {
            await loadObjects(accountId, { parentId: pageId });
          } else {
            await loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
          }
          await refreshDetailObjAfterSync(objectId);
          await refreshTemplateHashMap();
          return;
        }
        setSyncDialog((prev) => (prev ? { ...prev, result, loading: false } : null));
      } catch (err) {
        console.warn('[Workspace] Preview sync failed:', err);
        setSyncDialog(null);
        setSyncDialogOpenForObjectId(null);
      }
    },
    [accountId, previewSyncTemplate, applySyncTemplate, loadObjects, pageId, sectionFilter, refreshDetailObjAfterSync, refreshTemplateHashMap],
  );

  const handleConfirmSync = useCallback(async () => {
    if (!syncDialog || !accountId) return;
    setSyncDialog((prev) => (prev ? { ...prev, loading: true } : null));
    try {
      await applySyncTemplate(accountId, syncDialog.objectId);
      setSyncDialog(null);
      setSyncDialogOpenForObjectId(null);
      // 同步成功后对象 fingerprint 已更新；刷新对象列表。
      if (pageId) {
        await loadObjects(accountId, { parentId: pageId });
      } else {
        await loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
      await refreshDetailObjAfterSync(syncDialog.objectId);
      await refreshTemplateHashMap();
    } catch (err) {
      console.warn('[Workspace] Apply sync failed:', err);
      setSyncDialog((prev) => (prev ? { ...prev, loading: false } : null));
    }
  }, [syncDialog, accountId, applySyncTemplate, loadObjects, pageId, sectionFilter, refreshDetailObjAfterSync, refreshTemplateHashMap]);

  const handleDismissSync = useCallback(async (objectId: string, latestHash?: string) => {
    if (!latestHash) return;
    try {
      await ignoreTemplateSync(objectId, latestHash);
      // 后端已持久化 ignoredTemplateHash；刷新列表与指纹映射使提示条立即消失。
      if (accountId) {
        if (pageId) {
          await loadObjects(accountId, { parentId: pageId });
        } else {
          await loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
        }
        await refreshTemplateHashMap();
      }
    } catch (err) {
      console.warn('[Workspace] Ignore template sync failed:', err);
    }
  }, [ignoreTemplateSync, loadObjects, accountId, pageId, sectionFilter, refreshTemplateHashMap]);

  const handleRequestDismissSync = useCallback(
    (objectId: string, objectName: string, latestHash?: string) => {
      if (!latestHash) return;
      setSyncDialogOpenForObjectId(objectId);
      setDismissConfirm({ objectId, objectName, latestHash });
    },
    [],
  );

  const handleConfirmDismissSync = useCallback(() => {
    if (!dismissConfirm) return;
    handleDismissSync(dismissConfirm.objectId, dismissConfirm.latestHash);
    setDismissConfirm(null);
    setSyncDialogOpenForObjectId(null);
  }, [dismissConfirm, handleDismissSync]);

  const handleViewDeprecatedFields = useCallback(
    async (objectId: string, objectName: string) => {
      if (!accountId) return;
      setDeprecatedViewer({ objectId, objectName });
      try {
        const fields = await loadDeprecatedFields(accountId, objectId);
        setDeprecatedFields(fields);
      } catch (err) {
        console.warn('[Workspace] Load deprecated fields failed:', err);
        setDeprecatedFields([]);
      }
    },
    [accountId, loadDeprecatedFields],
  );

  return (
    <AppShell
      title={customPage?.name || activeCategoryLabel || t('objects')}
      onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <PageGuide
            pages={[
              {
                icon: LayoutList,
                title: t('common:guide_object_card_title') ?? '对象卡片指南',
                steps: [
                  {
                    icon: LayoutList,
                    title: t('common:guide_card_step1_title') ?? '卡片结构',
                    description:
                      t('common:guide_card_step1_desc') ??
                      '每张对象卡片上方显示模板图标、对象名称和所属类别标签。若对象绑定了模板，还会显示模板名称。右侧是快捷操作按钮：历史记录（时钟图标）、附件列表（回形针图标）、编辑（铅笔图标）、删除（垃圾桶图标）。卡片主体以标签形式展示对象的字段属性。',
                  },
                  {
                    icon: Shield,
                    title: t('common:guide_card_step2_title') ?? '敏感度颜色',
                    description:
                      t('common:guide_card_step2_desc') ??
                      '字段的外边框颜色代表其敏感度等级：🟢 绿色 = 公开（public），🔵 蓝色 = 内部（internal），🟠 琥珀色 = 敏感（sensitive），🔴 红色 = 关键（critical）。非公开字段的值会自动模糊处理，保护隐私。',
                  },
                  {
                    icon: Pencil,
                    title: t('common:guide_card_step3_title') ?? '交互操作',
                    description:
                      t('common:guide_card_step3_desc') ??
                      '点击卡片任意区域打开详情面板查看所有字段。右上角的按钮分别用于：查看历史版本快照、管理附件、编辑对象、删除对象。将文件直接拖到卡片上可快速添加附件。',
                  },
                ],
                helpLinks: [
                  {
                    title: t('common:guide_help_sensitivity') ?? '敏感度等级',
                    description:
                      t('common:guide_help_sensitivity_desc') ??
                      '了解不同敏感度等级的含义与安全策略',
                    href: '/help?id=sensitivity',
                  },
                  {
                    title: t('common:guide_help_objects') ?? '对象管理',
                    description:
                      t('common:guide_help_objects_desc') ??
                      '对象的创建、编辑、历史回溯与回收站管理',
                    href: '/help?id=objects',
                  },
                  {
                    title: t('common:guide_help_getting_started') ?? '快速开始',
                    description:
                      t('common:guide_help_getting_started_desc') ??
                      '了解 SoloSoul 基础操作与工作区布局',
                    href: '/help?id=getting-started',
                  },
                ],
              },
              {
                icon: LayoutTemplate,
                title: t('common:guide_template_title') ?? '对象模板指南',
                steps: [
                  {
                    icon: FileText,
                    title: t('common:guide_tpl_step1_title') ?? '什么是模板',
                    description:
                      t('common:guide_tpl_step1_desc') ??
                      '模板定义了对象的数据结构，包含一组字段。每个字段有名称、类型（文本、数字、日期、选择等）和敏感度等级。使用模板创建对象时，系统会自动生成对应的字段，无需手动逐一添加。',
                  },
                  {
                    icon: Settings,
                    title: t('common:guide_tpl_step2_title') ?? '管理模板',
                    description:
                      t('common:guide_tpl_step2_desc') ??
                      '前往设置 > 模板管理器，可查看所有可用模板、创建新模板、编辑已有模板的字段结构。您可以为模板选择图标、设置所属页面、调整字段顺序和敏感度。已废弃的字段可以恢复或永久清理。',
                  },
                  {
                    icon: Trash2,
                    title: t('common:guide_tpl_step3_title') ?? '使用模板',
                    description:
                      t('common:guide_tpl_step3_desc') ??
                      '在新建对象时，可以选择一个模板来快速填充数据。对象创建后仍可更改或替换模板。如果模板被删除，已有对象的字段数据不会丢失，模板名称会显示为删除线。',
                  },
                ],
                helpLinks: [
                  {
                    title: t('common:guide_help_templates') ?? '模板管理',
                    description:
                      t('common:guide_help_templates_desc') ??
                      '模板的创建、编辑、字段管理与废弃处理',
                    href: '/help?id=templates',
                  },
                  {
                    title: t('common:guide_help_create_object') ?? '创建对象',
                    description:
                      t('common:guide_help_create_object_desc') ??
                      '使用模板创建对象，快速录入结构化数据',
                    href: '/help?id=create-objects',
                  },
                  {
                    title: t('common:guide_help_objects') ?? '对象管理',
                    description:
                      t('common:guide_help_objects_desc') ??
                      '对象的创建、编辑、历史回溯与回收站管理',
                    href: '/help?id=objects',
                  },
                ],
              },
              {
                icon: Upload,
                title: t('common:drag_upload_guide_title') ?? '拖拽附件上传指南',
                steps: [
                  {
                    icon: LayoutList,
                    title: t('common:drag_guide_step1_title') ?? '对象卡片',
                    description:
                      t('common:drag_guide_step1_desc') ??
                      '在工作区列表中，直接将文件拖拽到任意对象的卡片上，即可为该对象添加附件。拖入时卡片会高亮提示。',
                  },
                  {
                    icon: Maximize2,
                    title: t('common:drag_guide_step2_title') ?? '对象详情',
                    description:
                      t('common:drag_guide_step2_desc') ??
                      '点击对象卡片打开详情面板，将文件拖入面板内的任意区域，即可快速附加到当前对象。',
                  },
                  {
                    icon: Paperclip,
                    title: t('common:drag_guide_step3_title') ?? '附件管理',
                    description:
                      t('common:drag_guide_step3_desc') ??
                      '在附件管理器弹窗中，直接将文件拖入窗口，即可批量上传多个附件。支持同时拖入多个文件。',
                  },
                ],
                helpLinks: [
                  {
                    title: t('common:guide_help_getting_started') ?? '快速开始',
                    description:
                      t('common:guide_help_getting_started_desc') ??
                      '了解 SoloSoul 基础操作与工作区布局',
                    href: '/help?id=getting-started',
                  },
                  {
                    title: t('common:guide_help_attachments') ?? '附件管理',
                    description:
                      t('common:guide_help_attachments_desc') ??
                      '附件的上传、下载、重命名与回收站管理',
                    href: '/help?id=attachments',
                  },
                  {
                    title: t('common:guide_help_sensitivity') ?? '敏感度等级',
                    description:
                      t('common:guide_help_sensitivity_desc') ??
                      '了解不同敏感度等级的含义与安全策略',
                    href: '/help?id=sensitivity',
                  },
                ],
              },
            ]}
          />
          <button
            className={`${buttonStyles.hideLabelOnMobile} ${workspaceStyles.createBtn}`}
            onClick={() => navigate(newObjectUrl)}
          >
            + <span className={buttonStyles.label}>{t('create')}</span>
          </button>
          {pageId && customPage && (
            <Button
              variant="danger-outline"
              size="sm"
              className={buttonStyles.hideLabelOnMobile}
              onClick={() => setConfirmPageDelete(true)}
              title={t('delete')}
            >
              <Trash size={ICON_SIZE.sm} /> <span className={buttonStyles.label}>{t('delete')}</span>
            </Button>
          )}
        </div>
      }
    >
      <PageContainer variant="medium" gap="default">
        <div
          style={{ display: 'contents' }}
          onMouseDown={(e) => {
            if (e.detail > 1) e.preventDefault();
          }}
        >
          <WorkspaceCategoryTabs
            sectionFilter={sectionFilter}
            pageId={pageId}
            customPages={customPages}
            activeCustomPages={activeCustomPages}
          />

          <Input
            placeholder={t('search_objects_placeholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onClear={() => setSearchQuery('')}
            prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
          />

          {isLoading && (
            <Card>
              <LoadingPlaceholder variant="elevated" minHeight={80} />
            </Card>
          )}
          {!isLoading && error && (
            <Card>
              <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>{error}</p>
            </Card>
          )}
          {!isLoading && !error && visibleObjects.length === 0 && (
            <Card>
              <p
                style={{
                  textAlign: 'center',
                  color: 'var(--text-secondary)',
                  padding: '24px 0',
                  fontSize: 'var(--text-sm)',
                }}
              >
                {searchQuery ? t('no_matching_objects') : t('no_objects')}
              </p>
            </Card>
          )}
          {!isLoading && visibleObjects.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
              {visibleObjects.map((obj) => (
                <WorkspaceObjectCard
                  key={obj.id}
                  obj={obj}
                  collectionLabel={resolveCollectionLabel(obj.collectionType)}
                  userTemplates={userTemplates}
                  snapshotCount={snapshotCounts[obj.id]}
                  attachmentCount={attachmentCounts[obj.id]}
                  templateHashMap={templateHashMap}
                  isSyncDialogOpen={syncDialogOpenForObjectId === obj.id}
                  onClick={() => setDetailObj(obj)}
                  onHistory={() =>
                    setHistoryObj({
                      id: obj.id,
                      name: obj.name,
                      collectionType: obj.collectionType,
                      templateId: obj.templateId || undefined,
                    })
                  }
                  onUploadComplete={refreshAttachmentCounts}
                  onAttachments={() => setAttachmentObjId(obj.id)}
                  onEdit={() => navigate(`/editor/${obj.id}`)}
                  onDelete={() => setConfirmDelete({ id: obj.id, name: obj.name })}
                  onSync={() => handleStartSync(obj.id, obj.name)}
                  onDismissSync={() =>
                    handleRequestDismissSync(
                      obj.id,
                      obj.name,
                      obj.templateId ? templateHashMap.get(obj.templateId) : undefined,
                    )
                  }
                />
              ))}
            </div>
          )}

          {/* Page delete confirmation dialog */}
          <ConfirmDeleteDialog
            isOpen={confirmPageDelete && !!pageId && !!customPage}
            title={t('object_delete_confirm_title')}
            body={t('object_delete_confirm_body', {
              name:
                (customPage?.name || '').length > 28
                  ? (customPage?.name || '').slice(0, 27) + '…'
                  : customPage?.name || '',
            })}
            confirmLabel={t('delete')}
            cancelLabel={t('cancel')}
            onCancel={() => setConfirmPageDelete(false)}
            onConfirm={async () => {
              setConfirmPageDelete(false);
              if (accountId && pageId) {
                await removeCustomPage(accountId, pageId);
                navigate('/');
              }
            }}
          />

          {/* Delete confirmation dialog */}
          {/* Object detail modal */}
          {detailObj && (
            <ObjectDetailModal
              object={detailObj}
              needsSync={
                (() => {
                  if (!templateHashMap || !detailObj.templateId || syncDialogOpenForObjectId === detailObj.id) return false;
                  return objectNeedsSync(detailObj, templateHashMap);
                })()
              }
              onClose={() => setDetailObj(null)}
              onEdit={() => {
                navigate(`/editor/${detailObj.id}`);
                setDetailObj(null);
              }}
              onDelete={() => {
                setConfirmDelete({ id: detailObj.id, name: detailObj.name });
                setDetailObj(null);
              }}
              onSyncTemplate={() => handleStartSync(detailObj.id, detailObj.name)}
              onDismissSync={() =>
                handleRequestDismissSync(
                  detailObj.id,
                  detailObj.name,
                  detailObj.templateId ? templateHashMap.get(detailObj.templateId) : undefined,
                )
              }
              onViewDeprecatedFields={() => handleViewDeprecatedFields(detailObj.id, detailObj.name)}
              onAttachmentsChange={refreshAttachmentCounts}
            />
          )}

          <ConfirmDeleteDialog
            isOpen={!!confirmDelete}
            title={t('object_delete_confirm_title')}
            body={t('object_delete_confirm_body', {
              name:
                (confirmDelete?.name || '').length > 28
                  ? (confirmDelete?.name || '').slice(0, 27) + '…'
                  : confirmDelete?.name || '',
            })}
            confirmLabel={t('delete')}
            cancelLabel={t('cancel')}
            onCancel={() => setConfirmDelete(null)}
            onConfirm={() => {
              if (confirmDelete) handleDelete(confirmDelete.id);
            }}
          />
        </div>
      </PageContainer>
      {historyObj &&
        (() => {
          const historyObjData = objects.find((o) => o.id === historyObj.id);
          const historyLabels = historyObjData?.propertyLabels;
          const historyFields = (historyObjData?.properties as Record<string, unknown>)
            ?.__fields as Record<string, { name: string }> | undefined;
          return (
            <HistoryViewer
              objectId={historyObj.id}
              objectName={historyObj.name}
              collectionType={historyObj.collectionType}
              onClose={() => setHistoryObj(null)}
              passwordVerify={passwordVerify}
              getFieldSensitivity={(fieldKey) =>
                getFieldSensitivity(historyObj.templateId, fieldKey, historyLabels)
              }
              isFieldDeprecated={(fieldKey) => isFieldDeprecated(historyObj.templateId, fieldKey)}
              getFieldName={(fieldKey) =>
                getFieldName(historyObj.templateId, fieldKey, historyFields)
              }
              fieldOrder={userTemplates
                .find((t) => t.id === historyObj.templateId)
                ?.properties.map((p) => p.id)}
            />
          );
        })()}
      {attachmentObjId && (
        <AttachmentViewer
          objectId={attachmentObjId}
          onClose={() => setAttachmentObjId(null)}
          onCountChange={refreshAttachmentCounts}
        />
      )}

      {/* 模板同步确认弹窗 */}
      {syncDialog && (
        <TemplateSyncConfirmDialog
          isOpen={true}
          result={syncDialog.result}
          loading={syncDialog.loading}
          onConfirm={handleConfirmSync}
          onCancel={() => {
            setSyncDialog(null);
            setSyncDialogOpenForObjectId(null);
          }}
        />
      )}

      {/* 忽略模板更新二次确认弹窗 */}
      {dismissConfirm && (
        <ConfirmDeleteDialog
          isOpen={true}
          title={t('editor:template_sync_dismiss_title')}
          body={t('editor:template_sync_dismiss_body')}
          confirmLabel={t('common:confirm')}
          cancelLabel={t('common:cancel')}
          onCancel={() => {
            setDismissConfirm(null);
            setSyncDialogOpenForObjectId(null);
          }}
          onConfirm={handleConfirmDismissSync}
        />
      )}

      {/* 历史字段查看器 */}
      {deprecatedViewer && (
        <DeprecatedFieldsViewer
          isOpen={true}
          objectName={deprecatedViewer.objectName}
          fields={deprecatedFields}
          onClose={() => {
            setDeprecatedViewer(null);
            setDeprecatedFields([]);
          }}
        />
      )}

      {/* Unified password verification dialog (detail panel + history cards) */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={() => {
          setShowPwDialog(false);
          pwResolveRef.current?.({ ok: false, method: 'password' });
        }}
        onVerify={async (password) => {
          const ok = await verifyVaultPassword(password);
          if (ok) pwResolveRef.current?.({ ok: true, method: 'password' });
          return ok;
        }}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        pinAccountId={accountId}
        onPinSuccess={() => {
          pwResolveRef.current?.({ ok: true, method: 'password' });
          setShowPwDialog(false);
        }}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </AppShell>
  );
}
