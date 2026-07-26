import { useEffect, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useToastError } from '@/hooks/useToastError';
import { isMobilePlatformSync } from '@/lib/platform';

import { invoke } from '@tauri-apps/api/core';
import type { OcrResult, OcrTierInfo, OcrModelStatus, MrzResult } from '@/lib/ipc';
import { OCR_MODEL_SERIES, OCR_MODEL_NOT_INSTALLED_PREFIX } from '@/lib/constants';
import { getTierLabel } from '@/lib/utils';
import { MrzResultCard } from '@/components/ocr/MrzResultCard';
import { PromptDialog } from '@/components/ui/PromptDialog';
import { Scan, Upload, Download, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

type ScanMode = 'general' | 'mrz';

export function OcrPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['ocr', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { createObject } = useObjectStore();
  const { onError, onSuccess } = useToastError();

  const initialFilePath = (location.state as { filePath?: string } | null)?.filePath || '';

  const [filePath, setFilePath] = useState(initialFilePath);
  const [result, setResult] = useState<OcrResult | null>(null);
  const [mrzResult, setMrzResult] = useState<MrzResult | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isNameDialogOpen, setIsNameDialogOpen] = useState(false);
  const [importNameDefault, setImportNameDefault] = useState('');
  const [pendingImportSource, setPendingImportSource] = useState<'ocr' | 'mrz' | null>(null);
  const [scanMode, setScanMode] = useState<ScanMode>('general');

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [activeTier, setActiveTier] = useState('small');
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');
  const isMobilePlatform = isMobilePlatformSync();

  /** 处理扫描错误：将后端返回的「模型未安装」前缀解析为国际化提示。 */
  const handleScanError = (err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    if (message.startsWith(`${OCR_MODEL_NOT_INSTALLED_PREFIX}:`)) {
      const tier = message.slice(OCR_MODEL_NOT_INSTALLED_PREFIX.length + 1) || activeTier;
      onError(new Error(t('ocr:scan_model_not_installed', { tier })), t('ocr:scan_failed'));
      return;
    }
    onError(err, t('ocr:scan_failed'));
  };

  /** 如果已知当前档位模型未安装，直接显示国际化提示。 */
  const guardActiveModelInstalled = (): boolean => {
    const status = statusMap[activeTier];
    if (status && !status.installed) {
      onError(
        new Error(t('ocr:scan_model_not_installed', { tier: activeTier })),
        t('ocr:scan_failed'),
      );
      return false;
    }
    return true;
  };

  const loadTiersAndStatus = async () => {
    try {
      setLoadingStatus(true);
      const [tierList, currentTier] = await Promise.all([
        invoke<OcrTierInfo[]>('ocr_list_available_tiers'),
        invoke<string>('ocr_get_active_tier'),
      ]);
      setTiers(tierList);
      setActiveTier(currentTier);

      const statuses: Record<string, OcrModelStatus> = {};
      await Promise.all(
        tierList.map(async (tier) => {
          const status = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: tier.tier });
          statuses[tier.tier] = status;
        }),
      );
      setStatusMap(statuses);
    } catch (e) {
      onError(e, t('ocr:load_status_failed'));
    } finally {
      setLoadingStatus(false);
    }
  };

  // P212: mount-only — intentionally omitting loadTiersAndStatus/onError/t to avoid re-run.
  // All deps are stable refs or setState setters; stale closures not an issue for this init call.
  useEffect(() => {
    loadTiersAndStatus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const getFileFilters = () => {
    // MRZ 与移动端均只支持图片格式（移动端 ML Kit 无法处理 PDF）
    if (scanMode === 'mrz' || isMobilePlatform) {
      return [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff'] }];
    }
    return [
      { name: 'Images & PDFs', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tiff', 'pdf'] },
    ];
  };

  const performScan = async (path: string) => {
    if (!guardActiveModelInstalled()) return;
    setIsScanning(true);
    setResult(null);
    setMrzResult(null);
    try {
      if (scanMode === 'mrz') {
        const res = await invoke<MrzResult | null>('ocr_scan_mrz', { filePath: path });
        if (res) {
          setMrzResult(res);
        } else {
          // MRZ not detected: fall back to general OCR
          const ocrRes = await invoke<OcrResult>('ocr_scan_image', { filePath: path });
          setResult(ocrRes);
          onSuccess(t('ocr:mrz_no_detected'));
        }
      } else {
        const res = await invoke<OcrResult>('ocr_scan_image', { filePath: path });
        setResult(res);
      }
    } catch (e) {
      handleScanError(e);
    } finally {
      setIsScanning(false);
    }
  };

  // 如果通过附件菜单传入文件路径，自动开始扫描。
  useEffect(() => {
    if (!initialFilePath) return;
    let cancelled = false;
    async function scan() {
      if (!guardActiveModelInstalled()) return;
      setIsScanning(true);
      setResult(null);
      setMrzResult(null);
      try {
        if (scanMode === 'mrz') {
          const res = await invoke<MrzResult | null>('ocr_scan_mrz', { filePath: initialFilePath });
          if (cancelled) return;
          if (res) {
            setMrzResult(res);
          } else {
            const ocrRes = await invoke<OcrResult>('ocr_scan_image', { filePath: initialFilePath });
            if (!cancelled) setResult(ocrRes);
          }
        } else {
          const res = await invoke<OcrResult>('ocr_scan_image', { filePath: initialFilePath });
          if (!cancelled) setResult(res);
        }
      } catch (e) {
        if (!cancelled) handleScanError(e);
      } finally {
        if (!cancelled) setIsScanning(false);
      }
    }
    scan();
    return () => {
      cancelled = true;
    };
    // P212: guardActiveModelInstalled/handleScanError/scanMode omitted intentionally —
    // they are stable functions; adding them would cause re-scan on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialFilePath]);

  const handleSelectFile = async () => {
    try {
      const { openWithPause } = await import('@/lib/dialog');
      const path = await openWithPause({
        filters: getFileFilters(),
        multiple: false,
        title:
          scanMode === 'mrz' || isMobilePlatform
            ? t('ocr:select_image_title')
            : t('ocr:select_file_title'),
      });
      if (path && typeof path === 'string') {
        setFilePath(path);
        await performScan(path);
      }
    } catch (e) {
      onError(e, t('ocr:select_image_failed'));
    }
  };

  const handleTakePhoto = async () => {
    // 启动相机前立即清空上次结果并显示加载状态，避免从相机返回后闪烁旧结果
    setResult(null);
    setMrzResult(null);
    setIsScanning(true);
    try {
      const { useAutoLockPauseStore } = await import('@/stores/autoLockPauseStore');
      const { pause, resume } = useAutoLockPauseStore.getState();
      pause();
      try {
        const path = await invoke<string | null>('mobile_ocr_take_photo');
        if (path) {
          setFilePath(path);
          await performScan(path);
        } else {
          // 相机取消或未返回路径，恢复空闲状态
          setIsScanning(false);
          onError(t('ocr:take_photo_no_image'), t('ocr:take_photo_failed'));
        }
      } catch (e) {
        setIsScanning(false);
        onError(e, t('ocr:take_photo_failed'));
      } finally {
        resume();
      }
    } catch (e) {
      setIsScanning(false);
      onError(e, t('ocr:take_photo_failed'));
    }
  };

  /** 生成 OCR 导入对象的默认名称：前缀 + 当前日期（YYYYMMDD） */
  const generateDefaultImportName = () => {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    const date = `${year}${month}${day}`;
    return t('ocr:import_default_name', { date, defaultValue: `OCR扫描结果${date}` });
  };

  const handleImportAsObject = (source: 'ocr' | 'mrz') => {
    if (!accountId) return;
    if (source === 'ocr' && !result) return;
    if (source === 'mrz' && !mrzResult) return;
    setPendingImportSource(source);
    setImportNameDefault(generateDefaultImportName());
    setIsNameDialogOpen(true);
  };

  const buildImportProperties = () => {
    if (pendingImportSource === 'ocr' && result) {
      return { ocrText: result.text };
    }
    if (pendingImportSource === 'mrz' && mrzResult) {
      const summary = [
        `${t('ocr:mrz_field_type')}: ${mrzResult.documentType} (${mrzResult.documentTypeSub})`,
        `${t('ocr:mrz_field_country')}: ${mrzResult.issuingCountry}`,
        `${t('ocr:mrz_field_number')}: ${mrzResult.documentNumber}`,
        `${t('ocr:mrz_field_nationality')}: ${mrzResult.nationality}`,
        `${t('ocr:mrz_field_dob')}: ${mrzResult.dateOfBirth}`,
        `${t('ocr:mrz_field_sex')}: ${mrzResult.sex}`,
        `${t('ocr:mrz_field_expiry')}: ${mrzResult.expiryDate}`,
        `${t('ocr:mrz_raw_lines')}:\n${mrzResult.rawLines.join('\n')}`,
      ].join('\n');
      return { ocrText: summary };
    }
    return {};
  };

  const handleConfirmImport = async (name: string) => {
    if (!accountId || !pendingImportSource) return;
    setIsNameDialogOpen(false);
    setIsImporting(true);
    try {
      await createObject({
        accountId,
        name,
        collectionType: 'document',
        properties: buildImportProperties(),
      });
      onSuccess(t('ocr:import_success'));
    } catch (e) {
      onError(e, t('ocr:import_failed'));
    } finally {
      setIsImporting(false);
      setPendingImportSource(null);
    }
  };

  const handleTierChange = async (tier: string) => {
    try {
      await invoke<void>('ocr_set_active_tier', { tier });
      setActiveTier(tier);
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleInstallBundled = async (tier: string) => {
    setInstallingTier(tier);
    try {
      await invoke<void>('ocr_install_bundled_model', { tier });
      await loadTiersAndStatus();
      onSuccess(t('ocr:install_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:install_failed', { tier }));
    } finally {
      setInstallingTier(null);
    }
  };

  const handleDownload = async (tier: string) => {
    if (!downloadUrl.trim()) {
      onError(new Error(t('ocr:download_url_required')), t('ocr:download_url_required'));
      return;
    }
    setDownloadingTier(tier);
    try {
      await invoke<void>('ocr_download_model', { tier, baseUrl: downloadUrl.trim() });
      await loadTiersAndStatus();
      onSuccess(t('ocr:download_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:download_failed', { tier }));
    } finally {
      setDownloadingTier(null);
    }
  };

  return (
    <AppShell
      title={t('ocr:title')}
      onBack={() => {
        const state = location.state as { fromHome?: boolean } | undefined;
        if (state?.fromHome) {
          navigate('/home');
        } else {
          navigate(-1);
        }
      }}
    >
      <PageContainer variant="wide" gap="default">
        {/* Model management（桌面端）；移动端使用系统 ML Kit，无需模型管理 */}
        {!isMobilePlatform && (
          <Card>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
              {t('ocr:model_title')}
            </h3>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div>
                <label
                  style={{
                    display: 'block',
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-secondary)',
                    marginBottom: 6,
                  }}
                >
                  {t('ocr:active_model_series', { model: OCR_MODEL_SERIES })}
                </label>
                <select
                  value={activeTier}
                  onChange={(e) => handleTierChange(e.target.value)}
                  disabled={loadingStatus}
                  style={{
                    width: '100%',
                    padding: '8px 10px',
                    fontSize: 'var(--text-body-sm)',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                  }}
                >
                  {tiers.map((tier) => {
                    const label = getTierLabel(t, tier);
                    return (
                      <option key={tier.tier} value={tier.tier}>
                        {label.name} — {label.description}
                      </option>
                    );
                  })}
                </select>
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {tiers.map((tier) => {
                  const status = statusMap[tier.tier];
                  const isInstalling = installingTier === tier.tier;
                  const isDownloading = downloadingTier === tier.tier;
                  return (
                    <div
                      key={tier.tier}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '10px 12px',
                        borderRadius: 8,
                        background: 'var(--bg-toolbar)',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
                        {status?.installed ? (
                          <CheckCircle size={ICON_SIZE.md} color="var(--accent-primary)" />
                        ) : status?.bundled ? (
                          <AlertCircle size={ICON_SIZE.md} color="var(--text-tertiary)" />
                        ) : (
                          <AlertCircle size={ICON_SIZE.md} color="var(--error)" />
                        )}
                        <div>
                          <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                            {getTierLabel(t, tier).name}
                          </div>
                          <div
                            style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}
                          >
                            {status?.installed
                              ? t('ocr:status_installed')
                              : status?.bundled
                                ? t('ocr:status_bundled')
                                : t('ocr:status_not_installed')}
                          </div>
                        </div>
                      </div>
                      <div style={{ display: 'flex', gap: 8 }}>
                        {status?.bundled && !status?.installed && (
                          <Button
                            size="sm"
                            onClick={() => handleInstallBundled(tier.tier)}
                            loading={isInstalling}
                          >
                            {t('ocr:install')}
                          </Button>
                        )}
                        {!status?.bundled && !status?.installed && (
                          <Button
                            size="sm"
                            onClick={() => handleDownload(tier.tier)}
                            loading={isDownloading}
                          >
                            <Download size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
                            {t('ocr:download')}
                          </Button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>

              {!statusMap['small']?.installed && !statusMap['small']?.bundled && (
                <div>
                  <label
                    style={{
                      display: 'block',
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-secondary)',
                      marginBottom: 6,
                    }}
                  >
                    {t('ocr:download_url_label')}
                  </label>
                  <input
                    type="text"
                    value={downloadUrl}
                    onChange={(e) => setDownloadUrl(e.target.value)}
                    placeholder={t('ocr:download_url_placeholder')}
                    style={{
                      width: '100%',
                      padding: '8px 10px',
                      fontSize: 'var(--text-body-sm)',
                      borderRadius: 8,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-elevated)',
                      color: 'var(--text-primary)',
                    }}
                  />
                </div>
              )}
            </div>
          </Card>
        )}

        {isMobilePlatform && (
          <Card>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 8 }}>
              {t('ocr:mobile_ocr_title')}
            </h3>
            <p
              style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}
            >
              {t('ocr:mobile_ocr_description')}
            </p>
          </Card>
        )}

        {/* Scan mode + action */}
        <Card>
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Scan
              size={ICON_SIZE['5xl']}
              style={{ marginBottom: 12, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 600, marginBottom: 4 }}>
              {t('ocr:title')}
            </h2>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                marginBottom: 16,
              }}
            >
              {t('ocr:description')}
            </p>

            {/* Mode toggle */}
            <div
              style={{
                display: 'inline-flex',
                gap: 4,
                padding: 4,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                marginBottom: 16,
              }}
            >
              <button
                onClick={() => {
                  setScanMode('general');
                  setResult(null);
                  setMrzResult(null);
                }}
                style={{
                  padding: '6px 14px',
                  borderRadius: 6,
                  border:
                    scanMode === 'general'
                      ? '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)'
                      : '1px solid transparent',
                  fontSize: 'var(--text-body-sm)',
                  cursor: 'pointer',
                  background:
                    scanMode === 'general'
                      ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                      : 'transparent',
                  color: scanMode === 'general' ? 'var(--accent-primary)' : 'var(--text-secondary)',
                  fontWeight: scanMode === 'general' ? 600 : 400,
                }}
              >
                {t('ocr:scan_mode_general')}
              </button>
              <button
                onClick={() => {
                  setScanMode('mrz');
                  setResult(null);
                  setMrzResult(null);
                }}
                style={{
                  padding: '6px 14px',
                  borderRadius: 6,
                  border:
                    scanMode === 'mrz'
                      ? '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)'
                      : '1px solid transparent',
                  fontSize: 'var(--text-body-sm)',
                  cursor: 'pointer',
                  background:
                    scanMode === 'mrz'
                      ? 'color-mix(in srgb, var(--accent-primary) 15%, transparent)'
                      : 'transparent',
                  color: scanMode === 'mrz' ? 'var(--accent-primary)' : 'var(--text-secondary)',
                  fontWeight: scanMode === 'mrz' ? 600 : 400,
                }}
              >
                {t('ocr:scan_mode_mrz')}
              </button>
            </div>

            <br />

            <div style={{ display: 'flex', gap: 8, justifyContent: 'center', alignItems: 'center' }}>
              <Button onClick={handleSelectFile} loading={isScanning}>
                {scanMode === 'mrz' || isMobilePlatform
                  ? t('ocr:select_image')
                  : t('ocr:select_image_or_pdf')}
              </Button>
              {isMobilePlatform && (
                <Button onClick={handleTakePhoto} loading={isScanning}>
                  {t('ocr:take_photo')}
                </Button>
              )}
            </div>
          </div>
        </Card>

        {isScanning && (
          <Card>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 10,
                padding: 24,
                color: 'var(--text-secondary)',
              }}
            >
              <Loader2 size={ICON_SIZE.lg} className="spin" />
              <span>{t('ocr:scanning')}</span>
            </div>
          </Card>
        )}

        {/* General OCR result */}
        {result && (
          <Card>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 12,
              }}
            >
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
                {t('ocr:result_title')}
              </h3>
              <Button size="sm" onClick={() => handleImportAsObject('ocr')} loading={isImporting}>
                <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} />{' '}
                {t('ocr:import_as_object')}
              </Button>
            </div>

            {result.boxes.length > 1 && (
              <div style={{ marginBottom: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {result.boxes.map((box, i) => (
                  <div
                    key={i}
                    style={{
                      display: 'flex',
                      gap: 12,
                      padding: '8px 10px',
                      borderRadius: 6,
                      background: 'var(--bg-toolbar)',
                      fontSize: 'var(--text-body-sm)',
                    }}
                  >
                    <span style={{ flex: 1, wordBreak: 'break-word' }}>{box.text}</span>
                    <span
                      style={{
                        fontSize: 'var(--text-badge)',
                        color: 'var(--text-tertiary)',
                        flexShrink: 0,
                      }}
                    >
                      {(box.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            )}

            {result.text && (
              <div
                style={{
                  padding: 12,
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 'var(--text-body-sm)',
                  lineHeight: 1.6,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 300,
                  overflowY: 'auto',
                }}
              >
                {result.text}
              </div>
            )}

            {result.boxes.length === 0 && !result.text && (
              <p
                style={{
                  textAlign: 'center',
                  color: 'var(--text-tertiary)',
                  padding: 24,
                  fontSize: 'var(--text-body-sm)',
                }}
              >
                {t('ocr:no_text')}
              </p>
            )}
          </Card>
        )}

        {/* MRZ result */}
        {mrzResult && (
          <Card>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 12,
              }}
            >
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
                {t('ocr:mrz_result_title')}
              </h3>
              <Button size="sm" onClick={() => handleImportAsObject('mrz')} loading={isImporting}>
                <Upload size={ICON_SIZE.sm} style={{ marginRight: 4 }} />{' '}
                {t('ocr:import_as_object')}
              </Button>
            </div>
            <MrzResultCard result={mrzResult} />
          </Card>
        )}

        <PromptDialog
          isOpen={isNameDialogOpen}
          title={t('ocr:import_name_dialog_title')}
          defaultValue={importNameDefault}
          placeholder={t('ocr:import_name_placeholder')}
          confirmLabel={t('common:confirm')}
          cancelLabel={t('common:cancel')}
          onConfirm={handleConfirmImport}
          onCancel={() => {
            setIsNameDialogOpen(false);
            setPendingImportSource(null);
          }}
        />
      </PageContainer>
    </AppShell>
  );
}
