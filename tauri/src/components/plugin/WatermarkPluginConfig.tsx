import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { downloadDir } from '@tauri-apps/api/path';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { FolderOpen } from 'lucide-react';
import { commands } from '@/lib/ipc';
import { ExpandableSection } from './shared/ExpandableSection';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { useAttachmentPageSort } from '@/hooks/useAttachmentPageSort';
import styles from './WatermarkPluginConfig.module.css';

interface WatermarkConfig {
  text: string;
  fontSize: number;
  color: [number, number, number];
  opacity: number;
  angle: number;
  position: 'center' | 'topLeft' | 'topRight' | 'bottomLeft' | 'bottomRight' | 'tile';
  tile: boolean;
  marginX: number;
  marginY: number;
}

interface AttachmentNode {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  pageId?: string;
  pageName: string;
  objectName: string;
  templateName?: string;
}

interface AttachmentPageGroup {
  pageId?: string;
  pageName: string;
  objects: Record<string, AttachmentNode[]>;
}

const DEFAULT_CONFIG: WatermarkConfig = {
  text: 'SoloSoul',
  fontSize: 72,
  color: [128, 128, 128],
  opacity: 0.3,
  angle: -45,
  position: 'center',
  tile: false,
  marginX: 0,
  marginY: 0,
};

const POSITION_I18N_KEYS: Record<WatermarkConfig['position'], string> = {
  center: 'watermark.position_center',
  topLeft: 'watermark.position_topLeft',
  topRight: 'watermark.position_topRight',
  bottomLeft: 'watermark.position_bottomLeft',
  bottomRight: 'watermark.position_bottomRight',
  tile: 'watermark.position_tile',
};

interface WatermarkPluginConfigProps {
  /** 当配置或附件选择发生变化时调用，返回可供 runPlugin 使用的 params */
  onParamsChange: (params: Record<string, string>) => void;
}

export function WatermarkPluginConfig({ onParamsChange }: WatermarkPluginConfigProps) {
  const { t } = useTranslation(['plugin', 'common', 'navigation']);

  const [config, setConfig] = useState<WatermarkConfig>(DEFAULT_CONFIG);
  const [attachments, setAttachments] = useState<AttachmentNode[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [loadingAttachments, setLoadingAttachments] = useState(false);
  const [outputDir, setOutputDir] = useState('');

  // ─── Load attachments ──────────────────────────────────────────────
  const loadAttachments = useCallback(async () => {
    setLoadingAttachments(true);
    try {
      const json = await commands.pluginListAttachments();
      const tree = JSON.parse(json) as {
        pages: Array<{
          pageId?: string;
          pageName: string;
          objects: Array<{
            objectId: string;
            objectName: string;
            templateName?: string;
            attachments: Array<{
              id: string;
              objectId: string;
              fileName: string;
              mimeType: string;
            }>;
          }>;
        }>;
      };
      const nodes: AttachmentNode[] = [];
      for (const page of tree.pages) {
        for (const obj of page.objects) {
          for (const att of obj.attachments) {
            nodes.push({
              id: `${att.objectId}/${att.id}`,
              objectId: att.objectId,
              fileName: att.fileName,
              mimeType: att.mimeType,
              pageId: page.pageId,
              pageName: page.pageName,
              objectName: obj.objectName,
              templateName: obj.templateName,
            });
          }
        }
      }
      setAttachments(nodes);
    } catch {
      // silent in sidebar
    } finally {
      setLoadingAttachments(false);
    }
  }, []);

  useEffect(() => {
    loadAttachments();
    downloadDir()
      .then(setOutputDir)
      .catch(() => setOutputDir(''));
  }, [loadAttachments]);

  // ─── Compute run params whenever config or selection changes ───────
  const selectedAttachments = useMemo(
    () => attachments.filter((a) => selectedIds.has(a.id)),
    [attachments, selectedIds],
  );

  // 使用 ref 缓存 onParamsChange，避免父组件重复渲染导致 effect 反复触发
  const onParamsChangeRef = useRef(onParamsChange);
  onParamsChangeRef.current = onParamsChange;

  useEffect(() => {
    const params: Record<string, string> = {
      watermarkConfig: JSON.stringify(config),
      outputDir,
      selectedAttachments: JSON.stringify(
        selectedAttachments.map((a) => ({
          objectId: a.objectId,
          attachmentId: a.id.split('/')[1],
          fileName: a.fileName,
          mimeType: a.mimeType,
          objectName: a.objectName,
          pageName: a.pageName,
          templateName: a.templateName ?? '',
        })),
      ),
    };
    onParamsChangeRef.current(params);
  }, [config, selectedAttachments, outputDir]);

  // ─── 当前配置摘要（实时更新） ──────────────────────────────────────
  const configSummaryParts = useMemo(() => {
    const lblText = t('watermark.summary_text', { defaultValue: '文本' });
    const lblFont = t('watermark.summary_font_size', { defaultValue: '字号' });
    const lblColor = t('watermark.summary_color', { defaultValue: '颜色' });
    const lblOpacity = t('watermark.summary_opacity', { defaultValue: '透明度' });
    const lblAngle = t('watermark.summary_angle', { defaultValue: '角度' });
    const lblPosition = t('watermark.summary_position', { defaultValue: '位置' });
    const posLabel = t(POSITION_I18N_KEYS[config.position], { defaultValue: config.position });
    const tileLabel = t(POSITION_I18N_KEYS.tile, { defaultValue: 'Tile' });
    const tilePart = config.tile ? ` · ${tileLabel}` : '';
    return {
      textPart: `${lblText}: ${config.text}`,
      restPart: ` · ${lblFont}: ${config.fontSize} · ${lblColor}: rgb(${config.color.join(',')}) · ${lblOpacity}: ${(config.opacity * 100).toFixed(0)}% · ${lblAngle}: ${config.angle}° · ${lblPosition}: ${posLabel}${tilePart}`,
    };
  }, [config, t]);

  // ─── Attachment grouping ───────────────────────────────────────────
  const groupedPages = useMemo(() => groupByPageObject(attachments), [attachments]);
  const sortedPages = useAttachmentPageSort(groupedPages);

  // ─── Handlers ──────────────────────────────────────────────────────
  const handleToggleAttachment = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleChangeOutputDir = async () => {
    const dir = await openDialog({ directory: true });
    if (dir) setOutputDir(dir);
  };

  const handleColorChange = (index: number, value: number) => {
    setConfig((c) => {
      const color: [number, number, number] = [...c.color] as [number, number, number];
      color[index] = value;
      return { ...c, color };
    });
  };

  return (
    <div className={styles.configArea}>
      {/* ── 输出目录（始终可见的简洁行） ──────────────────────────────── */}
      <div className={styles.outputDirRow}>
        <span className={styles.outputDirLabel}>
          {t('watermark.output_dir', { defaultValue: '输出目录' })}:
        </span>
        <span className={styles.outputDirValue} title={outputDir}>
          {outputDir || t('common:loading', { defaultValue: '加载中...' })}
        </span>
        <BadgeIconButton
          Icon={FolderOpen}
          onClick={handleChangeOutputDir}
          title={t('watermark.change_output_dir', { defaultValue: '更改输出目录' })}
        />
      </div>

      {/* ── 当前配置摘要（始终可见，文本部分过长时单独截断） ────────── */}
      <div
        className={styles.configSummary}
        title={`${configSummaryParts.textPart}${configSummaryParts.restPart}`}
      >
        <span className={styles.configSummaryText}>{configSummaryParts.textPart}</span>
        <span className={styles.configSummaryRest}>{configSummaryParts.restPart}</span>
      </div>

      {/* ── 水印配置 Section ────────────────────────────────────────── */}
      <ExpandableSection
        title={t('watermark_config_sidebar', {
          defaultValue: '水印配置',
        })}
      >
        <div className={styles.configBody}>
          <label className={styles.field}>
            <span>{t('watermark.text', { defaultValue: '水印文本' })}</span>
            <input
              type="text"
              value={config.text}
              onChange={(e) => setConfig((c) => ({ ...c, text: e.target.value }))}
            />
          </label>

          <label className={styles.field}>
            <span>{t('watermark.font_size', { defaultValue: '字号' })}</span>
            <input
              type="number"
              min={8}
              max={300}
              value={config.fontSize}
              onChange={(e) => setConfig((c) => ({ ...c, fontSize: Number(e.target.value) }))}
            />
          </label>

          <label className={styles.field}>
            <span>{t('watermark.opacity', { defaultValue: '透明度' })}</span>
            <div className={styles.rangeRow}>
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={config.opacity}
                onChange={(e) => setConfig((c) => ({ ...c, opacity: Number(e.target.value) }))}
              />
              <span className={styles.rangeValue}>{(config.opacity * 100).toFixed(0)}%</span>
            </div>
          </label>

          <label className={styles.field}>
            <span>{t('watermark.angle', { defaultValue: '旋转角度' })}</span>
            <input
              type="number"
              min={-180}
              max={180}
              value={config.angle}
              onChange={(e) => setConfig((c) => ({ ...c, angle: Number(e.target.value) }))}
            />
          </label>

          <label className={styles.field}>
            <span>{t('watermark.color', { defaultValue: '颜色 (R,G,B)' })}</span>
            <div className={styles.colorRow}>
              {([0, 1, 2] as const).map((i) => (
                <input
                  key={i}
                  type="number"
                  min={0}
                  max={255}
                  value={config.color[i]}
                  onChange={(e) => handleColorChange(i, Number(e.target.value))}
                />
              ))}
            </div>
          </label>

          <label className={styles.field}>
            <span>{t('watermark.position', { defaultValue: '位置' })}</span>
            <select
              value={config.position}
              onChange={(e) =>
                setConfig((c) => ({
                  ...c,
                  position: e.target.value as WatermarkConfig['position'],
                }))
              }
            >
              {(Object.keys(POSITION_I18N_KEYS) as WatermarkConfig['position'][]).map((value) => (
                <option key={value} value={value}>
                  {t(POSITION_I18N_KEYS[value], { defaultValue: value })}
                </option>
              ))}
            </select>
          </label>

          <label
            className={styles.fieldInline}
            onClick={() => setConfig((c) => ({ ...c, tile: !c.tile }))}
          >
            <SelectCheckbox checked={config.tile} />
            <span>{t('watermark.tile', { defaultValue: '平铺水印' })}</span>
          </label>
        </div>
      </ExpandableSection>

      {/* ── 附件选择 Section ────────────────────────────────────────── */}
      <ExpandableSection
        title={t('select_attachments_sidebar', {
          defaultValue: '附件选择',
        })}
        count={selectedAttachments.length > 0 ? selectedAttachments.length : undefined}
      >
        {loadingAttachments ? (
          <div className={styles.loading}>{t('common:loading', { defaultValue: '加载中...' })}</div>
        ) : attachments.length === 0 ? (
          <div className={styles.empty}>
            {t('watermark.no_attachments', {
              defaultValue: '没有可用的图片/PDF 附件',
            })}
          </div>
        ) : (
          <div className={styles.tree}>
            {sortedPages.map((page) => (
              <div key={page.pageName} className={styles.treePage}>
                <div className={styles.treePageName}>
                  {page.pageId
                    ? page.pageName
                    : t(`navigation:${page.pageName}`, {
                        defaultValue: page.pageName,
                      })}
                </div>
                {Object.entries(page.objects).map(([objectName, atts]) => (
                  <div key={objectName} className={styles.treeObject}>
                    <div className={styles.treeObjectName}>{objectName}</div>
                    {atts.map((a) => (
                      <label
                        key={a.id}
                        className={styles.treeItem}
                        onClick={() => handleToggleAttachment(a.id)}
                      >
                        <SelectCheckbox checked={selectedIds.has(a.id)} />
                        <span className={styles.treeItemName}>{a.fileName}</span>
                        <span className={styles.treeItemMime}>{a.mimeType}</span>
                      </label>
                    ))}
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </ExpandableSection>
    </div>
  );
}

// ─── Helper: group attachments by page → object ────────────────────────
function groupByPageObject(attachments: AttachmentNode[]): AttachmentPageGroup[] {
  const map = new Map<string, { pageId?: string; objects: Map<string, AttachmentNode[]> }>();
  for (const a of attachments) {
    if (!map.has(a.pageName)) {
      map.set(a.pageName, { pageId: a.pageId, objects: new Map() });
    }
    const entry = map.get(a.pageName)!;
    if (!entry.objects.has(a.objectName)) entry.objects.set(a.objectName, []);
    entry.objects.get(a.objectName)!.push(a);
  }
  return Array.from(map.entries()).map(([pageName, entry]) => ({
    pageId: entry.pageId,
    pageName,
    objects: Object.fromEntries(entry.objects),
  }));
}
