import { useCallback, useEffect, useMemo, useState } from 'react';
import { downloadDir } from '@tauri-apps/api/path';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Settings2, Paperclip, Play, Eye, Download, FolderOpen, Check, Loader2 } from 'lucide-react';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { BadgeIconButton } from '@/components/ui/BadgeIconButton';
import { commands } from '@/lib/ipc';
import { pluginCommands, type PluginLogLine, type WatermarkResultItem, type WatermarkResultPayload } from '@/lib/plugin';
import { useToastError } from '@/hooks/useToastError';
import { open } from '@tauri-apps/plugin-shell';
import { save, open as openDialog } from '@tauri-apps/plugin-dialog';
import { copyFile } from '@tauri-apps/plugin-fs';
import styles from './WatermarkPluginPage.module.css';
import { ICON_SIZE } from '@/lib/iconSizes';

const PLUGIN_ID = 'com.solosoul.official.watermark';

interface AttachmentNode {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  pageId?: string;
  pageName: string;
  objectName: string;
}

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

const POSITION_OPTIONS: { value: WatermarkConfig['position']; label: string }[] = [
  { value: 'center', label: '居中' },
  { value: 'topLeft', label: '左上角' },
  { value: 'topRight', label: '右上角' },
  { value: 'bottomLeft', label: '左下角' },
  { value: 'bottomRight', label: '右下角' },
  { value: 'tile', label: '平铺' },
];

export function WatermarkPluginPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['plugin', 'common']);
  const { onError } = useToastError();

  const [config, setConfig] = useState<WatermarkConfig>(DEFAULT_CONFIG);
  const [showConfigCard, setShowConfigCard] = useState(false);
  const [showAttachCard, setShowAttachCard] = useState(false);

  const [attachments, setAttachments] = useState<AttachmentNode[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [loadingAttachments, setLoadingAttachments] = useState(false);

  const [outputDir, setOutputDir] = useState('');
  const [running, setRunning] = useState(false);
  const [logs, setLogs] = useState<PluginLogLine[]>([]);
  const [results, setResults] = useState<WatermarkResultItem[]>([]);

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
            });
          }
        }
      }
      setAttachments(nodes);
    } catch (err) {
      onError(err, t('plugin:watermark.load_attachments_failed', { defaultValue: '加载附件失败' }));
    } finally {
      setLoadingAttachments(false);
    }
  }, [onError, t]);

  useEffect(() => {
    loadAttachments();
    downloadDir().then(setOutputDir).catch(() => setOutputDir(''));
  }, [loadAttachments]);

  const selectedAttachments = useMemo(
    () => attachments.filter((a) => selectedIds.has(a.id)),
    [attachments, selectedIds],
  );

  const configSummary = useMemo(() => {
    const pos = POSITION_OPTIONS.find((p) => p.value === config.position)?.label ?? config.position;
    return `文本: ${config.text} · 字号: ${config.fontSize} · 颜色: rgb(${config.color.join(',')}) · 透明度: ${(config.opacity * 100).toFixed(0)}% · 角度: ${config.angle}° · 位置: ${pos}${config.tile ? ' · 平铺' : ''}`;
  }, [config]);

  const handleToggleAttachment = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleRun = async () => {
    if (selectedAttachments.length === 0) {
      onError(new Error('请先选择附件'), t('plugin:watermark.no_attachments', { defaultValue: '未选择附件' }));
      return;
    }
    if (!outputDir) {
      onError(new Error('请选择输出目录'), t('plugin:watermark.no_output_dir', { defaultValue: '未选择输出目录' }));
      return;
    }

    setRunning(true);
    setLogs([]);
    setResults([]);

    const params: Record<string, string> = {
      watermarkConfig: JSON.stringify(config),
      selectedAttachments: JSON.stringify(
        selectedAttachments.map((a) => ({
          objectId: a.objectId,
          attachmentId: a.id.split('/')[1],
          fileName: a.fileName,
          mimeType: a.mimeType,
        })),
      ),
      outputDir,
    };

    try {
      const res = await pluginCommands.run(PLUGIN_ID, params, (event) => {
        if (event.eventType === 'log') {
          try {
            const line = JSON.parse(event.jsonData) as PluginLogLine;
            setLogs((prev) => [...prev, line]);
          } catch {
            setLogs((prev) => [
              ...prev,
              { id: String(Date.now()), level: 'info', message: event.jsonData, timestamp: Date.now() },
            ]);
          }
        } else if (event.eventType === 'result') {
          try {
            const payload = JSON.parse(event.jsonData) as { type: string };
            if (payload.type === 'watermark_result') {
              const wm = payload as unknown as WatermarkResultPayload;
              setResults(wm.items);
            }
          } catch {
            // ignore
          }
        }
      });

      if (res.exitCode !== 0) {
        onError(new Error(`插件退出码 ${res.exitCode}`), t('plugin:watermark.run_failed', { defaultValue: '水印添加失败' }));
      }
    } catch (err) {
      onError(err, t('plugin:watermark.run_failed', { defaultValue: '水印添加失败' }));
    } finally {
      setRunning(false);
    }
  };

  const handlePreview = async (path: string) => {
    try {
      const fileUrl = new URL(path.replace(/\\/g, '/'), 'file://').href;
      await open(fileUrl);
    } catch (err) {
      onError(err, t('plugin:watermark.preview_failed', { defaultValue: '预览失败' }));
    }
  };

  const handleDownload = async (item: WatermarkResultItem) => {
    try {
      const dest = await save({ defaultPath: item.fileName });
      if (!dest) return;
      await copyFile(item.outputPath, dest);
    } catch (err) {
      onError(err, t('plugin:watermark.download_failed', { defaultValue: '下载失败' }));
    }
  };

  const handleChangeOutputDir = async () => {
    const dir = await openDialog({ directory: true });
    if (dir) setOutputDir(dir);
  };

  return (
    <AppShell
      title={t('plugin:watermark.title', { defaultValue: '附件水印添加' })}
      onBack={() => navigate('/plugins')}
    >
      <PageContainer>
        <Card className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('plugin:watermark.config_section', { defaultValue: '插件配置' })}</h2>

          <div className={styles.configRow}>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowConfigCard((v) => !v)}
              disabled={running}
            >
              <Settings2 size={ICON_SIZE.sm} />
              {t('plugin:watermark.watermark_config', { defaultValue: '水印配置' })}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowAttachCard((v) => !v)}
              disabled={running}
            >
              <Paperclip size={ICON_SIZE.sm} />
              {t('plugin:watermark.select_attachments', { defaultValue: '附件选择' })}
            </Button>
          </div>

          {config && !showConfigCard && (
            <div className={styles.summary}>{configSummary}</div>
          )}

          {selectedAttachments.length > 0 && (
            <div className={styles.selectedList}>
              <div className={styles.selectedTitle}>
                {t('plugin:watermark.selected_attachments', { defaultValue: '已选择附件' })}
              </div>
              {selectedAttachments.map((a) => (
                <div key={a.id} className={styles.selectedItem}>
                  <span className={styles.selectedName}>{a.fileName}</span>
                  <span className={styles.selectedMeta}>
                    {a.objectName} · {a.pageName}
                  </span>
                </div>
              ))}
            </div>
          )}

          {showConfigCard && (
            <div className={styles.cardBody}>
              <label className={styles.field}>
                <span>{t('plugin:watermark.text', { defaultValue: '水印文本' })}</span>
                <input
                  type="text"
                  value={config.text}
                  onChange={(e) => setConfig((c) => ({ ...c, text: e.target.value }))}
                />
              </label>
              <label className={styles.field}>
                <span>{t('plugin:watermark.font_size', { defaultValue: '字号' })}</span>
                <input
                  type="number"
                  min={8}
                  max={300}
                  value={config.fontSize}
                  onChange={(e) => setConfig((c) => ({ ...c, fontSize: Number(e.target.value) }))}
                />
              </label>
              <label className={styles.field}>
                <span>{t('plugin:watermark.opacity', { defaultValue: '透明度' })}</span>
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.05}
                  value={config.opacity}
                  onChange={(e) => setConfig((c) => ({ ...c, opacity: Number(e.target.value) }))}
                />
                <span>{(config.opacity * 100).toFixed(0)}%</span>
              </label>
              <label className={styles.field}>
                <span>{t('plugin:watermark.angle', { defaultValue: '旋转角度' })}</span>
                <input
                  type="number"
                  min={-180}
                  max={180}
                  value={config.angle}
                  onChange={(e) => setConfig((c) => ({ ...c, angle: Number(e.target.value) }))}
                />
              </label>
              <label className={styles.field}>
                <span>{t('plugin:watermark.color', { defaultValue: '颜色 (R,G,B)' })}</span>
                <div className={styles.colorRow}>
                  {([0, 1, 2] as const).map((i) => (
                    <input
                      key={i}
                      type="number"
                      min={0}
                      max={255}
                      value={config.color[i]}
                      onChange={(e) =>
                        setConfig((c) => {
                          const color: [number, number, number] = [...c.color] as [number, number, number];
                          color[i] = Number(e.target.value);
                          return { ...c, color };
                        })
                      }
                    />
                  ))}
                </div>
              </label>
              <label className={styles.field}>
                <span>{t('plugin:watermark.position', { defaultValue: '位置' })}</span>
                <select
                  value={config.position}
                  onChange={(e) =>
                    setConfig((c) => ({ ...c, position: e.target.value as WatermarkConfig['position'] }))
                  }
                >
                  {POSITION_OPTIONS.map((p) => (
                    <option key={p.value} value={p.value}>
                      {p.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className={styles.fieldInline}>
                <input
                  type="checkbox"
                  checked={config.tile}
                  onChange={(e) => setConfig((c) => ({ ...c, tile: e.target.checked }))}
                />
                <span>{t('plugin:watermark.tile', { defaultValue: '平铺水印' })}</span>
              </label>
              <div className={styles.cardActions}>
                <Button variant="tertiary" size="sm" onClick={() => setConfig(DEFAULT_CONFIG)}>
                  {t('common:reset', { defaultValue: '重置' })}
                </Button>
                <Button variant="primary" size="sm" onClick={() => setShowConfigCard(false)}>
                  <Check size={ICON_SIZE.sm} />
                  {t('common:save', { defaultValue: '保存' })}
                </Button>
              </div>
            </div>
          )}

          {showAttachCard && (
            <div className={styles.cardBody}>
              {loadingAttachments ? (
                <div className={styles.empty}>{t('common:loading', { defaultValue: '加载中...' })}</div>
              ) : attachments.length === 0 ? (
                <div className={styles.empty}>{t('plugin:watermark.no_attachments', { defaultValue: '没有可用的图片/PDF 附件' })}</div>
              ) : (
                <div className={styles.tree}>
                  {groupByPageObject(attachments).map(([pageName, objects]) => (
                    <div key={pageName} className={styles.treePage}>
                      <div className={styles.treePageName}>{pageName}</div>
                      {Object.entries(objects).map(([objectName, atts]) => (
                        <div key={objectName} className={styles.treeObject}>
                          <div className={styles.treeObjectName}>{objectName}</div>
                          {atts.map((a) => (
                            <label key={a.id} className={styles.treeItem}>
                              <input
                                type="checkbox"
                                checked={selectedIds.has(a.id)}
                                onChange={() => handleToggleAttachment(a.id)}
                              />
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
              <div className={styles.cardActions}>
                <Button variant="primary" size="sm" onClick={() => setShowAttachCard(false)}>
                  <Check size={ICON_SIZE.sm} />
                  {t('common:save', { defaultValue: '保存' })}
                </Button>
              </div>
            </div>
          )}

          <div className={styles.runRow}>
            <Button
              variant="primary"
              size="lg"
              loading={running}
              disabled={selectedAttachments.length === 0}
              onClick={handleRun}
            >
              {running ? (
                <Loader2 size={ICON_SIZE.sm} className={styles.spin} />
              ) : (
                <Play size={ICON_SIZE.sm} />
              )}
              {t('plugin:watermark.add_watermark', { defaultValue: '添加水印' })}
            </Button>
          </div>
        </Card>

        {(logs.length > 0 || running) && (
          <Card className={styles.section}>
            <h2 className={styles.sectionTitle}>{t('plugin:watermark.logs', { defaultValue: '插件日志' })}</h2>
            <div className={styles.logs}>
              {logs.map((log) => (
                <div key={log.id} className={`${styles.logLine} ${styles[log.level]}`}>
                  <span className={styles.logLevel}>{log.level.toUpperCase()}</span>
                  <span className={styles.logMessage}>{log.message}</span>
                </div>
              ))}
              {running && logs.length === 0 && (
                <div className={styles.empty}>{t('plugin:watermark.running', { defaultValue: '正在处理...' })}</div>
              )}
            </div>
          </Card>
        )}

        {results.length > 0 && (
          <Card className={styles.section}>
            <div className={styles.resultHeader}>
              <h2 className={styles.sectionTitle}>{t('plugin:watermark.results', { defaultValue: '插件结果' })}</h2>
              <div className={styles.outputDirRow}>
                <span className={styles.outputDirLabel}>
                  {t('plugin:watermark.output_dir', { defaultValue: '下载路径' })}:
                </span>
                <span className={styles.outputDir}>{outputDir}</span>
                <BadgeIconButton
                  Icon={FolderOpen}
                  onClick={handleChangeOutputDir}
                  title={t('plugin:watermark.change_output_dir', { defaultValue: '更改下载路径' })}
                />
              </div>
            </div>
            <div className={styles.resultList}>
              {results.map((item) => (
                <div key={`${item.objectId}-${item.attachmentId}`} className={styles.resultItem}>
                  <div className={styles.resultInfo}>
                    <span className={styles.resultName}>{item.fileName}</span>
                    <span className={styles.resultMime}>{item.mimeType}</span>
                  </div>
                  <div className={styles.resultActions}>
                    <BadgeIconButton
                      Icon={Eye}
                      onClick={() => handlePreview(item.outputPath)}
                      title={t('plugin:watermark.preview', { defaultValue: '预览' })}
                    />
                    <BadgeIconButton
                      Icon={Download}
                      onClick={() => handleDownload(item)}
                      title={t('plugin:watermark.download', { defaultValue: '下载' })}
                    />
                  </div>
                </div>
              ))}
            </div>
          </Card>
        )}
      </PageContainer>
    </AppShell>
  );
}

function groupByPageObject(
  attachments: AttachmentNode[],
): Array<[string, Record<string, AttachmentNode[]>]> {
  const map = new Map<string, Map<string, AttachmentNode[]>>();
  for (const a of attachments) {
    if (!map.has(a.pageName)) map.set(a.pageName, new Map());
    const objMap = map.get(a.pageName)!;
    if (!objMap.has(a.objectName)) objMap.set(a.objectName, []);
    objMap.get(a.objectName)!.push(a);
  }
  return Array.from(map.entries()).map(([page, objMap]) => [page, Object.fromEntries(objMap)]);
}
